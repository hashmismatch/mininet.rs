use std::{any::{Any, TypeId}, collections::HashMap};


#[derive(Default)]
pub struct Extras {
    extras: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}
impl Extras {
    pub fn get<'a, T: 'static>(&'a self) -> Option<&'a T>
    where
        T: Any + Send + Sync + 'static,
    {
        let ty = TypeId::of::<T>();
        
        if let Some(v)  = self.extras.get(&ty) {
            (&**v as &dyn Any).downcast_ref::<T>()
        } else {
            None
        }
    }

    pub fn insert<T>(&mut self, val: T)
        where
            T: Any + Send + Sync + 'static
    {
        let ty = TypeId::of::<T>();
        self.extras.insert(ty, Box::new(val));
    }

    pub fn get_mut<'a, T>(&'a mut self) -> Option<&'a mut T>
    where
        T: Any + Send + Sync + 'static,
    {
        let ty = TypeId::of::<T>();

        if let Some(v) = self.extras.get_mut(&ty) {
            (&mut **v as &mut dyn Any).downcast_mut::<T>()
        } else {
            None
        }
    }

    pub fn take<T>(&mut self) -> Option<Box<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let ty = TypeId::of::<T>();

        if let Some(v) = self.extras.remove(&ty) {
            v.downcast::<T>().ok()
        } else {
            None
        }
    }
}

#[test]
fn test_extras() {
    let mut e = Extras::default();

    let v = e.get::<u32>();
    assert_eq!(None, v);
    let v = e.get_mut::<u32>();
    assert_eq!(None, v);
    e.insert(5u32);
    let v = e.get::<u32>();
    assert_eq!(Some(&5), v);
    let v = e.take::<u32>().unwrap();
    assert_eq!(5, *v);
}
