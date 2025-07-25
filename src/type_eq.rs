pub trait TypeEq {
    const EQ: bool;
}
impl<T> TypeEq for (T, T) {
    const EQ: bool = true;
}
impl<T, U> TypeEq for (T, U) {
    const EQ: bool = false;
}
