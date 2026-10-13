//! Product front ends render typed values, never read a destination's bytes.
pub mod headless;
pub mod interactive;

/// The selected product presentation, priced before the local host adopts resources.
pub enum Front {
    /// One headless outcome and its output policy.
    Headless(headless::Headless),
    /// The line-mode typed live view.
    Interactive(interactive::Interactive),
}
