//pub type HookFn<T> = dyn Fn() -> T + Send + Sync + 'static;
pub trait HookFn<T>: Fn() -> T + Send + Sync + 'static {}
impl<F, T> HookFn<T> for F
where
    F: Fn() -> T + Send + Sync + 'static
{}

/// Hooks to be called on threadpool events
pub struct Hooks {
    /// Called when a thread is created, must provide a name for the thread
    name_fn: Box<dyn HookFn<String>>,
    /// Called when a thread is started, before it starts working
    on_start_fn: Option<Box<dyn HookFn<()>>>,
    /// Called when a thread is stopped, before it stops working
    on_stop_fn: Option<Box<dyn HookFn<()>>>,
    /// Called when a thread is parked
    on_park_fn: Option<Box<dyn HookFn<()>>>,
    /// Called when a thread is unparked
    on_unpark_fn: Option<Box<dyn HookFn<()>>>,
    /// Called before a thread executes a task
    before_work_fn: Option<Box<dyn HookFn<()>>>,
    /// Called after a thread executes a task
    after_work_fn: Option<Box<dyn HookFn<()>>>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            name_fn: Box::new(|| "Unnamed".to_string()),
            on_start_fn: None,
            on_stop_fn: None,
            on_park_fn: None,
            on_unpark_fn: None,
            before_work_fn: None,
            after_work_fn: None,
        }
    }

    /// Set the name function
    pub fn set_name_fn(&mut self, name_fn: impl HookFn<String>) -> &mut Self {
        self.name_fn = Box::new(name_fn);
        self
    }

    /// Set the on_start function
    pub fn set_on_start_fn(&mut self, on_start_fn: impl HookFn<()>) -> &mut Self {
        self.on_start_fn = Some(Box::new(on_start_fn));
        self
    }

    /// Set the on_stop function
    pub fn set_on_stop_fn(&mut self, on_stop_fn: impl HookFn<()>) -> &mut Self {
        self.on_stop_fn = Some(Box::new(on_stop_fn));
        self
    }

    /// Set the on_park function
    pub fn set_on_park_fn(&mut self, on_park_fn: impl HookFn<()>) -> &mut Self {
        self.on_park_fn = Some(Box::new(on_park_fn));
        self
    }

    /// Set the on_unpark function
    pub fn set_on_unpark_fn(&mut self, on_unpark_fn: impl HookFn<()>) -> &mut Self {
        self.on_unpark_fn = Some(Box::new(on_unpark_fn));
        self
    }

    /// Set the before_work function
    pub fn set_before_work_fn(&mut self, before_work_fn: impl HookFn<()>) -> &mut Self {
        self.before_work_fn = Some(Box::new(before_work_fn));
        self
    }

    /// Set the after_work function
    pub fn set_after_work_fn(&mut self, after_work_fn: impl HookFn<()>) -> &mut Self {
        self.after_work_fn = Some(Box::new(after_work_fn));
        self
    }

    /// Call the name function
    pub(crate) fn call_name_fn(&self) -> String {
        (self.name_fn)()
    }

    /// Call the on_start function
    pub(crate) fn call_on_start_fn(&self) {
        if let Some(ref f) = self.on_start_fn {
            f();
        }
    }

    /// Call the on_stop function
    pub(crate) fn call_on_stop_fn(&self) {
        if let Some(ref f) = self.on_stop_fn {
            f();
        }
    }

    /// Call the on_park function
    pub(crate) fn call_on_park_fn(&self) {
        if let Some(ref f) = self.on_park_fn {
            f();
        }
    }

    /// Call the on_unpark function
    pub(crate) fn call_on_unpark_fn(&self) {
        if let Some(ref f) = self.on_unpark_fn {
            f();
        }
    }

    /// Call the before_work function
    pub(crate) fn call_before_work_fn(&self) {
        if let Some(ref f) = self.before_work_fn {
            f();
        }
    }

    /// Call the after_work function
    pub(crate) fn call_after_work_fn(&self) {
        if let Some(ref f) = self.after_work_fn {
            f();
        }
    }
}
