// Copyright 2015 Philipp Korber
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy
// of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software 
// distributed under the License is distributed on an "AS IS" BASIS, 
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. 
// See the License for the specific language governing permissions and 
// limitations under the License.

//alloc is needed for BoxAny, witch willl most like be removed in the future
//this is not a problem because the methodes of BoxAny will the most like
//become part of Any so that this lib can be updated by removing the 
//import and feature statement
#![feature(alloc)]
//this is needed 'cause Associated Types in combination with e.g. Index are not yet complety stable
#![feature(core)]
//for now it is unstable
#![unstable(feature="alloc,core")]

use std::rc::Rc;
use std::cell::RefCell;
use std::cell::RefMut;
use std::hash::Hash;

//import and reexport dyn_property
pub use dyn_property::DynProperty;
pub use dyn_property::UndefinedProperty;
pub use inner_dyn_object::InnerDynObject;

///! 
///! InnerDynObject is a kind of dynamic objects witch allows
///! creating and deleting properties at runtime.
///! This includs runtime type checks over genereic functions
///! so that the rest of your programm don't has to care mutch
///! about. Neverless this has to backdrawings:
///!   1. Accessing the variables allways returns a Result
///!   2. it has to own the data
///!   3. it's slower. If you have a group of variables putng
///!      them into a POD rust object and then puting it into
///!      InnerDynObject might be preferable

mod dyn_property;
mod inner_dyn_object;

//guard types, not should be used in boxes?
//FIXME maybe add the TypeID as parameter to the function call
type SetPropertyGuard<'a, Key> = FnMut(&'a mut InnerDynObject<Key>, &'a Key) -> bool;
type CreatePropertyGuard<'a, Key> = FnMut(&'a mut InnerDynObject<Key>, &'a Key) -> bool;
type RemovePropertyGuard<'a ,Key> = FnMut(&'a mut InnerDynObject<Key>, &'a Key) -> bool;
type AccessPropertyGuardRef<'a, Key> = FnMut(&'a InnerDynObject<Key>, &'a Key) -> bool;
type AccessPropertyGuardMut<'a, Key> = FnMut(&'a mut InnerDynObject<Key>, &'a Key) -> bool;


pub struct DynObject<Key> {
    inner: Rc<RefCell<InnerDynObject<Key>>>
}


impl<Key> DynObject<Key> where Key: Eq+Hash {

    pub fn new<T>() -> DynObject<T>
        where T: Eq + Hash
    {
        let x =  InnerDynObject::<T>::new();
        let cell = RefCell::new(x);
        let rc = Rc::new(cell);
        let weak_ref = rc.downgrade();
        rc.borrow_mut().set_uplink(weak_ref);
        DynObject { inner: rc }
    }
    
    /// create a DynObject from a reference to a InnerDynObject 
    ///
    /// This InnerDynObject has have a uplink, witch is the case if it origins in
    /// another DynObject instance. From then on both instances will share the same
    /// InnerDynObject
    ///
    /// # Panics
    /// if the uplink is not set or invalide
    /// 
    pub fn create_from<'a, T>(innerdyn: &'a InnerDynObject<T>) -> DynObject<T>
        where T: Eq + Hash
    {
        match innerdyn.get_uplink() {
            &Some(ref weak) => {
                match weak.upgrade() {
                    Some(full_rc) => DynObject {
                        inner: full_rc
                    },
                    None => panic!("refered InnerDynObject was a zomby")
                }
            },
            &None => panic!("refered InnerInnerDynObject was not created by a DynObject") 
        }
    }

    /// aquire the DynObject to perform operations on it
    ///
    /// # Panics
    /// if someone else aquired it and didn't relase it jet
    /// (by droping the returned RefMut, witch is often done 
    /// implicitly)
    pub fn aquire(&mut self) -> RefMut<InnerDynObject<Key>> {
        self.inner.borrow_mut()
    }
}

impl<T> Clone for DynObject<T> where T: Eq+Hash {

    fn clone(&self) -> Self {
        DynObject {
            inner: self.inner.clone()
        }
    }
}


#[cfg(test)]
mod test_dyn_object {
    #![allow(unused_variables)]

    use super::DynObject;

    fn create_dummy() -> DynObject<&'static str> {
        DynObject::<&'static str>::new()
    }
    
    #[test]
    fn aquire_should_not_panic_if_only_on_instance_exists() {
        let mut x = create_dummy();
        let data = x.aquire();
    }

    #[test]
    #[should_fail]
    fn aquire_multiple_times_should_panic() {
        let mut x = create_dummy();
        let mut obj_ref_2 = x.clone();
        let data = x.aquire();
        let data2 = obj_ref_2.aquire();
    }

    #[test]
    fn aquire_multiple_times_after_relasing_each_should_not_fail() {
        let mut x = create_dummy();
        {
            let data = x.aquire();
        }
        let data2 = x.aquire();
    }
    
    fn set_data(mut target: DynObject<&'static str>, value: i32) {
        target.aquire().create_property(&"hallo", Box::new(value));
    }

    #[test]
    fn mutiple_cloned_dyn_object_should_share_the_same_core() {
        let value = 23i32;
        let mut obj1 = create_dummy(); 
        set_data(obj1.clone(), value);
        let obj = obj1.aquire();
        match obj["hallo"].as_ref::<i32>() {
            Some(data) => assert_eq!(data, &value),
            None => panic!("type mismatch, error in test or other class")
        }
    }

    #[test]
    #[should_fail]
    fn create_from_should_panic_if_no_uplink_exists() {
        use super::InnerDynObject;
        let obj = InnerDynObject::<&'static str>::new();
        DynObject::<&'static str>::create_from(&obj);
    }

    #[test]
    fn create_from_should_work_with_a_valid_reference() {
        let mut obj = create_dummy();
        let obj_ref = obj.aquire();
        let obj2 = DynObject::<&'static str>::create_from(&obj_ref);
        //no panic -> ok
    }

    #[test]
    fn instances_created_with_create_from_should_share_state() {
        let mut obj = create_dummy();
        let mut obj2 = {
            let mut obj_ref = obj.aquire();
            let res = DynObject::<&'static str>::create_from(&obj_ref);
            obj_ref.create_property("hallo", Box::new(22i32));
            res
        };
        let obj2_ref = obj2.aquire();
        assert!(obj2_ref.exists_property(&"hallo"));
    }
}
