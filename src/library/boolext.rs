/// Helper trait for converting from `bool` to `Option`.
pub trait BoolExt: Sized {
    /// If the bool is true, return the result of `func`, wrapped in `Some`;
    /// otherwise return `None`.
    fn then<T, F: FnOnce() -> T>(self, func: F) -> Option<T>;

    fn then_some<T>(self, value: T) -> Option<T> {
        self.then(move || value)
    }
}

impl BoolExt for bool {
    fn then<T, F: FnOnce() -> T>(self, func: F) -> Option<T> {
        if self {
            Some(func())
        } else {
            None
        }
    }
}
