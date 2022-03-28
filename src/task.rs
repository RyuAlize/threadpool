pub trait Task: Send + 'static {
    fn run(self);
}