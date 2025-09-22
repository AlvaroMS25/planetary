use std::{mem::MaybeUninit, ptr::NonNull, sync::Mutex};

use crate::{task::state::Snapshot, JoinResult};

use super::{park::Parker, runnable::Runnable, state::State, vtable::VTable};

#[repr(C)]
/// A task that can be run by the executor.
pub struct Task<T, R> {
    header: Header,
    function: MaybeUninit<T>,
    output: MaybeUninit<JoinResult<R>>,
}

/// Header of the task, used to interact with the task
pub struct Header {
    vtable: &'static VTable,
    pub(crate) state: State,
    parker: Mutex<Parker>
}

pub struct TypeErasedTask {
    pub(crate) header: NonNull<Header>
}

unsafe impl Send for TypeErasedTask {}
unsafe impl Sync for TypeErasedTask {}

impl<T, R> Task<T, R>
where
    T: Runnable<Output = R>,
{
    pub fn new(runnable: T) -> Self {
        Self {
            header: Header {
                vtable: vtable::vtable::<T>(),
                state: State::new(),
                parker: Default::default(),
            },
            function: MaybeUninit::new(runnable),
            output: MaybeUninit::uninit(),
        }
    }

    pub fn erase(self) -> TypeErasedTask {
        let header = Box::into_raw(Box::new(self)).cast::<Header>();
        
        unsafe {
            // set the executor alive flag
            (*header).state.set(State::EXECUTOR_ALIVE, true);
        }

        TypeErasedTask {
            header: NonNull::new(header).unwrap()
        }
    }
}

impl Header {
    fn run(this: NonNull<Self>) {
        unsafe {
            let run_fn = this.as_ref().vtable.run;
            run_fn(this.cast());

            this.as_ref().wake();
        }
    }

    fn abort(this: NonNull<Self>) {
        unsafe {
            let abort_fn = this.as_ref().vtable.abort;
            abort_fn(this.cast());

            this.as_ref().wake();
        }
    }

    pub fn try_dealloc(this: NonNull<Self>) -> bool {
        unsafe {
            let dealloc_fn = this.as_ref().vtable.drop;
            dealloc_fn(this.cast())
        }
    }

    fn wake(&self) {
        self.parker.lock().unwrap_or_else(|s| s.into_inner())
                .take()
                .wake();
    }

    pub unsafe fn try_get_output(this: NonNull<Self>, dest: *mut ()) {
        unsafe {
            let get_output_fn = this.as_ref().vtable.take_output;
            get_output_fn(this.cast(), dest)
        }
    }

    #[inline(always)]
    pub fn parker(&self) -> &Mutex<Parker> {
        &self.parker
    }

    pub fn mark_aborted(&self) {
        self.state.set(State::ABORTED, true);
    }

    pub fn state_snapshot(&self) -> Snapshot {
        self.state.snapshot()
    }
}

impl TypeErasedTask {
    pub fn run(self) {
        Header::run(self.header);
    }
}

impl Drop for TypeErasedTask {
    fn drop(&mut self) {
        unsafe {
            let header = self.header.as_ref();
            // type erased task is only held by the executor, so update the state
            // to reflect the drop
            header.state.set(State::EXECUTOR_ALIVE, false);

            Header::try_dealloc(self.header);
        }
    }
}


mod vtable {
    use std::{mem::MaybeUninit, panic::{catch_unwind, AssertUnwindSafe}, ptr::NonNull};

    use crate::{task::{runnable::Runnable, state::State, vtable::VTable}, JoinResult};

    use super::{Header, Task};

    pub fn vtable<T>() -> &'static VTable 
    where
        T: Runnable
    {
        &VTable {
            run: run::<T>,
            abort: abort,
            drop: try_dealloc::<T>,
            take_output: try_get_output::<T>,
        }
    }

    unsafe fn run<T>(ptr: NonNull<()>) 
    where
        T: Runnable
    {
        let header = unsafe {
            ptr.cast::<Header>().as_ref()
        };

        assert!(!header.state.get(State::RUNNING));
        assert!(!header.state.get(State::FINISHED));

        if header.state.get(State::ABORTED) {
            return;
        }

        header.state.set(State::RUNNING, true);

        let mut ptr = ptr.cast::<Task<T, T::Output>>();

        let task = unsafe {
            ptr.as_mut()
        };

        let runnable = unsafe {
            std::mem::replace(&mut task.function, MaybeUninit::uninit())
                .assume_init()
        };

        let result = catch_unwind(AssertUnwindSafe(|| runnable.run()));

        task.output = MaybeUninit::new(result);

        task.header.state.set(State::RUNNING, false);
        task.header.state.set(State::FINISHED, true);
        task.header.state.set(State::OUTPUT_READY, true);

        task.header.wake();
    }

    unsafe fn abort(ptr: NonNull<()>) {
        let header = unsafe {
            ptr.cast::<Header>().as_ref()
        };

        // just set aborted flag
        header.state.set(State::ABORTED, true);
    }

    unsafe fn try_dealloc<T>(ptr: NonNull<()>) -> bool 
    where
        T: Runnable
    {
        let header = unsafe {
            ptr.cast::<Header>().as_ref()
        };

        if header.state.get(State::HANDLE_ALIVE) 
            || header.state.get(State::EXECUTOR_ALIVE) 
        {
            return false;
        }

        // drop the task
        let mut task = ptr.cast::<Task<T, T::Output>>();
        
        unsafe  {
            let task_mut = task.as_mut();

            // if finished flag is not set, the function is still there
            if !task_mut.header.state.get(State::FINISHED) {
                task_mut.function.assume_init_drop();
            }

            // if output flag is set, the output is there
            if task_mut.header.state.get(State::OUTPUT_READY) 
                && !task_mut.header.state.get(State::OUTPUT_TAKEN)
            {
                task_mut.output.assume_init_drop();
            }

            // drop the task
            drop(Box::from_raw(task.as_ptr()));
        }

        true
    }

    unsafe fn try_get_output<T>(ptr: NonNull<()>, dest: *mut ())
    where
        T: Runnable
    {
        let header = unsafe {
            ptr.cast::<Header>().as_ref()
        };

        if !header.state.get(State::OUTPUT_READY)
            || header.state.get(State::OUTPUT_TAKEN)
        {
            return;
        }

        header.state.set(State::OUTPUT_TAKEN, true);

        let dest = dest.cast::<Option<JoinResult<T::Output>>>();

        let mut task = ptr.cast::<Task<T, T::Output>>();

        unsafe {
            let output = std::mem::replace(&mut task.as_mut().output, MaybeUninit::uninit())
                .assume_init();

            *dest = Some(output);
            task.as_mut().header.state.set(State::OUTPUT_TAKEN, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{task::state::State, JoinResult};

    use super::Task;

    #[test]
    fn runnable() {
        print!("Runnable ran")
    }

    #[test]
    pub fn create_drop_task() {
        let task = Task::new(runnable);
    }

    #[test]
    pub fn create_drop_erased() {
        let task = Task::new(runnable);
        let erased = task.erase();
    }

    #[test]
    pub fn create_run_erased() {
        let task = Task::new(runnable);
        let erased = task.erase();
        erased.run();
    }

    #[test]
    pub fn create_take_output() {
        let task = Task::new(|| {
            runnable();
            "foo"
        });

        let erased = task.erase();
        let header = erased.header;

        unsafe {
            // this is technically not a handle, but put this flag to avoid
            // deallocating the task when running
            header.as_ref().state.set(State::HANDLE_ALIVE, true);
        }
        erased.run();

        let mut output = Option::<JoinResult<&'static str>>::None;

        unsafe {
            let get_output_fn = header.as_ref().vtable.take_output;
            get_output_fn(header.cast(), &mut output as *mut _ as *mut ());
        }

        assert!(output.is_some());
        assert_eq!(output.unwrap().unwrap(), "foo");

        let header_ref = unsafe { header.as_ref() };

        assert!(!header_ref.state.get(State::RUNNING));
        assert!(header_ref.state.get(State::FINISHED));
        assert!(header_ref.state.get(State::OUTPUT_READY));
        assert!(header_ref.state.get(State::OUTPUT_TAKEN));
        assert!(!header_ref.state.get(State::EXECUTOR_ALIVE));

        unsafe {
            header.as_ref().state.set(State::HANDLE_ALIVE, false);

            let drop_fn = header.as_ref().vtable.drop;
            drop_fn(header.cast());
        }
    }
}
