pub trait Get<T>
where
    T: serde::Serialize + Sized + Default,
{
    fn get() -> T {
        T::default()
    }
}

pub trait Post<T>
where
    T: serde::Serialize + Sized + Default,
{
    fn post() -> T {
        T::default()
    }
}

pub trait Put<T>
where
    T: serde::Serialize + Sized + Default,
{
    fn put() -> T {
        T::default()
    }
}

pub trait Patch<T>
where
    T: serde::Serialize + Sized + Default,
{
    fn patch() -> T {
        T::default()
    }
}
