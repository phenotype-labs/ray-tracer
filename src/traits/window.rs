/// Window lifecycle and event handling abstraction
pub trait WindowContext {
    /// Request the window to redraw
    fn request_redraw(&self);

    /// Get the inner size of the window in physical pixels
    fn inner_size(&self) -> (u32, u32);

    /// Get the scale factor for HiDPI displays
    fn scale_factor(&self) -> f64;

    /// Set cursor visibility
    fn set_cursor_visible(&self, visible: bool);
}
