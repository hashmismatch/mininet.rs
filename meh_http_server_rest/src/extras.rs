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
            Some((&**v as &dyn Any).downcast_ref::<T>().unwrap())
        } else {
            None
        }
    }

    pub fn get_mut<'a, T>(&'a mut self) -> &'a mut T
    where
        T: Default + Any + Send + Sync + 'static,
    {
        let ty = TypeId::of::<T>();
        let v = self.extras
            .entry(ty)
            .or_insert_with(|| Box::new(T::default()));
        (&mut **v as &mut dyn Any).downcast_mut::<T>().unwrap()
    }
}

#[test]
fn test_extras() {
    let mut e = Extras::default();

    let v = e.get::<u32>();
    assert_eq!(None, v);
    let v = e.get_mut::<u32>();
    assert_eq!(*v, 0);
    *v = 5;
    let v = e.get::<u32>();
    assert_eq!(Some(&5), v);
}