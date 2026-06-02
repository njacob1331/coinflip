pub struct Cache<T> {
    inner: Option<T>,
}

impl<T> Cache<T> {
    pub fn new() -> Self {
        Self { inner: None }
    }

    pub fn with_init(value: T) -> Self {
        Self { inner: Some(value) }
    }

    pub fn read(&self) -> Option<&T> {
        self.inner.as_ref()
    }

    pub fn read_mut(&mut self) -> Option<&mut T> {
        self.inner.as_mut()
    }

    pub fn write(&mut self, value: T) {
        self.inner = Some(value)
    }

    pub fn write_if<P>(&mut self, value: T, predicate: P) -> bool
    where
        P: FnOnce(&T, &T) -> bool,
    {
        let Some(inner) = self.inner.as_ref() else {
            self.write(value);
            return true;
        };

        if predicate(inner, &value) {
            self.write(value);
            true
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_none()
    }

    pub fn clear(&mut self) -> Option<T> {
        self.inner.take()
    }
}
