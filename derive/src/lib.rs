// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # The BitcoinMW derive crate
//! The `bmw_derive` crate implements the derive macros and derive attributes which are used
//! within BitcoinMW.

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

// errorkind section
mod errorkind;
use crate::errorkind::do_derive_errorkind;

/// The [`ErrorKind()`] attribute macro produces an implementation of the [`bmw_base::ErrorKind`]
/// trait based on the spcecified enumeration. This attribute macro makes it easy for crates and
/// modules to define their own error kinds that are compatible with the [`bmw_base::Error`] struct
/// so that all crates can share the same error but yet offer the flexibility to define their own
/// kinds of errors. In the example below some of the values are commented and some are not. If the
/// value is documented that comment will be included in the front part of the error string when
/// it's returned. If not specified a default value is used which is the Snake case of the error
/// variant with underscores replaced by spaces. So, for IllegalState, the error would be, "illegal
/// state: user entered string".
/// # Examples
///```
/// // this can be done by including bmw_core instead of both derive
/// // and base in higher level crates.
/// use bmw_derive::*;
/// use bmw_base::*;
/// // include our errors for convenience
/// use MyCrateErrorKinds::*;
///
/// // define kinds of errors for this module
/// #[ErrorKind]
/// pub enum MyCrateErrorKinds {
///     IllegalState,
///     /// arr index err
///     ArrayIndexOutOfBounds,
///     MyOtherErr,
///     SomethingElse,
/// }
///
/// // generate a 'SomethingElse' error
/// fn gen_err() -> Result<(), Error> {
///     err!(SomethingElse, "something else happened!")
/// }
///
/// // generate an 'ArrayIndexOutOfBounds' error
/// fn gen_err2() -> Result<(), Error> {
///     err!(ArrayIndexOutOfBounds, "tried to access index 2 when len is 1")
/// }
///
/// fn main() -> Result<(), Error> {
///     let err = gen_err().unwrap_err();
///     // assert value
///     assert_eq!(
///         err.kind().to_string(),
///         "something else: something else happened!".to_string()
///     );
///
///     let err2 = gen_err2().unwrap_err();
///     // assert value
///     assert_eq!(
///         err2.kind().to_string(),
///         "arr index err: tried to access index 2 when len is 1".to_string()
///     );
///
///     // assertions by error kind are possible as well
///     assert_eq!(err.kind(), kind!(SomethingElse));
///     assert_eq!(err2.kind(), kind!(ArrayIndexOutOfBounds));
///
///     Ok(())
/// }
///```
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[allow(non_snake_case)]
pub fn ErrorKind(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_errorkind(attr, item)
}

// config section
mod config;
use crate::config::do_derive_configurable;

/// The [`Configurable`] derive macro produces an implementation of the [`bmw_base::Configurable`]
/// trait based on the specified struct. This derive macro makes it easy for crates and modules to
/// define their own configurable data types and configure other structures with these data types.
/// This macro is used by the [`class()`] and [`debug_class()`] attribute as well.
///
/// # Examples
///```
/// use bmw_derive::*;
/// use bmw_base::*;
///
/// // define a configurable
/// #[derive(Configurable, Clone)]
/// struct MyStruct {
///    my_val: usize,
///    my_other_value: String,
/// }
///
/// impl Default for MyStruct {
///     fn default() -> Self {
///         Self {
///             my_val: 123,
///             my_other_value: "test".to_string(),
///         }
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     // configure with default values and assert
///     let my_struct = configure!(MyStruct, MyStructOptions, vec![])?;
///     assert_eq!(my_struct.my_val, 123);
///     assert_eq!(my_struct.my_other_value, "test".to_string());
///
///     // configure with non-default values and assert
///     // note that values are passed in as &str instead of String.
///     // conversions are automatic.
///     let my_struct = configure!(
///         MyStruct,
///         MyStructOptions,
///         vec![MyVal(111), MyOtherValue("abc")]
///     )?;
///     assert_eq!(my_struct.my_val, 111);
///     assert_eq!(my_struct.my_other_value, "abc".to_string());
///
///     Ok(())
/// }
///```
/// # Configuration enumertion uses Pascal Case
/// Notice that the my_value field was configured with the MyVal(111) variant.
/// The variants are automatically derived in Pascal case.
/// # Convenience of importing
/// Notice in the above example, only bmw_base and bmw_derive were used yet the other values were
/// all imported by the macros. In higher level crates, bmw_core may be used to avoid having these
/// two use statements.
#[proc_macro_derive(Configurable, attributes(required, passthrough))]
#[proc_macro_error]
#[cfg(not(tarpaulin_include))]
pub fn derive_configurable(strm: TokenStream) -> TokenStream {
	do_derive_configurable(strm, false)
}

#[proc_macro_derive(DebugConfigurable, attributes(required, passthrough))]
#[proc_macro_error]
#[cfg(not(tarpaulin_include))]
pub fn derive_debug_configurable(strm: TokenStream) -> TokenStream {
	do_derive_configurable(strm, true)
}

// ser section
mod ser;
use crate::ser::do_derive_serializable;

/// The [`Serializable`] derive macro produces an implementation of the [`bmw_base::Serializable`]
/// trait based on the specified struct or enum. This derive macro makes it easy for crates and
/// modules to define their own serializable data types for use in their data types and transport
/// layers.
///
/// # Examples
///```
/// use bmw_base::*;
/// use bmw_derive::*;
/// use std::fmt::Debug;
///
/// // derive Serializable along with other derive attributes
/// #[derive(Serializable, PartialEq, Debug, Clone)]
/// struct Employee {
///     id: usize,
///     name: String,
/// }
///
/// // serialize and deserialize the Employee record. Return the deserialized value
/// fn test_ser<S: Serializable + Debug + PartialEq>(employee_in: S) -> Result<S, Error> {
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &employee_in)?;
///     let employee_out: S = deserialize(&mut &v[..])?;
///     Ok(employee_out)
/// }
///
/// fn main() -> Result<(), Error> {
///     // create an employee record
///     let employee_in = Employee { id: 1234, name: "Sam Carter".to_string() };
///     // send it to be serialized then deserialized
///     let ret = test_ser(employee_in.clone())?;
///     // confirm values are equal
///     assert_eq!(ret.id, employee_in.id);
///     assert_eq!(ret.name, employee_in.name);
///     Ok(())
/// }
///```
#[proc_macro_derive(Serializable)]
#[cfg(not(tarpaulin_include))]
pub fn derive_serializable(strm: TokenStream) -> TokenStream {
	do_derive_serializable(strm)
}

// document section
mod document;
use crate::document::do_derive_document;

/// The [`document()`] attribute macro produces documentation for a specified function or macro. It
/// is used by the [`class()`] and [`debug_class()`] attributes as well. To document parameters,
/// first, the document attribute must be placed outside the function or macro, then the following
/// documentation annotations are available:
///
/// * param - documents a parameter
/// * return - documents a return type
/// * error - documents and error
/// * see - points to another piece of documentation
/// * deprecated - indicates that a function or macro is deprecated
///
/// The output, such as the example below, will be formatted nicely with input parameters in a
/// table and links to all the linked values and in a consistent format.
///
/// # Examples
///```
/// use bmw_base::*;
/// use bmw_derive::document;
///
/// #[document]
/// /// This line will go to the top
/// /// @deprecated
/// /// @param name the name of the user
/// /// @param code for this user
/// /// @param address an optional address for this user
/// /// @return the user id code for this user
/// /// @error bmw_base::CoreErrorKind::IllegalArgument if the code is invalid
/// /// @error bmw_base::CoreErrorKind::IllegalState
/// /// @error bmw_base::CoreErrorKind::IO if an i/o error occurs
/// /// @see bmw_base::Error
/// /// @see bmw_derive::class()
/// /// # Examples
/// ///
/// /// note: examples go to the bottom
/// ///```
/// /// use bmw_base::*;
/// /// fn main() -> Result<(), Error> { Ok(()) }
/// ///```
/// pub fn my_fn(name: String, code: u32, address: Option<String>) -> Result<u64, Error> {
///     Ok(0)
/// }
///
/// fn main() -> Result<(), Error> {
///     Ok(())
/// }
///```
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn document(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_document(attr, item)
}

// class section
mod class;
use crate::class::do_derive_class;

/// The [`crate::class()`] attribute defines a composite data type that incorporates other types, functions,
/// and views. Classes offer functionalities reminiscent of those found in other programming languages
/// while upholding Rust's fundamental principles. Classes can implement traits and serve as fields within a
/// struct or as variants of an enum if one of the boxed forms are generated.
/// # Hello World Class
/// Below is a simple example of a class. The comments explain each section of the class.
///```
/// // use bmw_core::*; instead of the following three lines
/// use bmw_base::*;
/// use bmw_derive::*;
/// use bmw_base as bmw_core;
///
/// // create a 'hello world' class.
/// #[class {
///     // constants are defined with a default value which can be overwritten
///     const val: String = "hello world!".to_string();
///
///     // this defines a list of views for this class. This example has
///     // one view, the 'hello_world' view.
///     [hello_world]
///     // this function must be implemented in a separate impl block
///     fn display(&self) -> String;
/// }] impl MyClass {} // class name
///
/// // this trait impl defines how the class is built. In this case there
/// // no vars just constants so Self is empty.
/// impl MyClassVarBuilder for MyClassVar {
///     fn builder(constants: &MyClassConst) -> Result<Self, Error> {
///         Ok(Self {})
///     }
/// }
///
/// // a separate block may be used to implement the defined functions
/// // in the class block.
/// impl MyClass {
///     // implement display to just return a clone of the constant value
///     // defined in the class.
///     fn display(&self) -> String {
///         self.constants().val.clone()
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     // create a class instance with the hello_world view
///     // val will be initialized to the default value 'hello world!'.
///     let x = hello_world!()?;
///     assert_eq!(x.display(), "hello world!".to_string());
///
///     // this time overwrite the default with 'hello planet!'
///     let y = hello_world!(Val("hello planet!"))?;
///     assert_eq!(y.display(), "hello planet!".to_string());
///     Ok(())
/// }
///```
/// # Using Multiple views for testing
/// One of the use cases for multiple views of the class is for testing. In the example below, we
/// create a regular view and a test view.
///
///```
/// // use bmw_core::*; instead of the following three lines
/// use bmw_base::*;
/// use bmw_derive::*;
/// use bmw_base as bmw_core;
///
/// #[class {
///     // create a 'var' with a type boolean to use for testing.
///     var is_test: bool;
///
///     // define two views for the bark function.
///     [dog, test]
///     fn bark(&self) -> Result<String, Error>;
///
///     // define a debug function that only the test view can execute
///     [test]
///     fn debug(&mut self);
/// }] impl DogTest {}
///
/// impl DogTestVarBuilder for DogTestVar {
///     fn builder(constants: &DogTestConst) -> Result<Self, Error> {
///         // the builder function needs to initialize the is_test var.
///         // we set it to false by default
///         Ok(Self { is_test: false })
///     }
/// }
///
/// impl DogTest {
///     fn bark(&self) -> Result<String, Error> {
///         // check if the is_test flag is set.
///         // if so, we return an error
///         // otherwise return the expected value
///         // we access the value via the 'vars()' function
///         // which provides getters for all vars.
///         if *self.vars().get_is_test() {
///             err!(CoreErrorKind::IllegalState, "test")
///         } else {
///             Ok("ruff!".to_string())
///         }
///     }
///
///     fn debug(&mut self) {
///         // set the is_test value using the .vars_mut() function.
///         // which provides getters and mutters for all vars.
///         *self.vars_mut().get_mut_is_test() = true;
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     let dog = dog!()?;
///     assert_eq!(dog.bark(), Ok("ruff!".to_string()));
///     let mut test = test!()?;
///     assert_eq!(test.bark(), Ok("ruff!".to_string()));
///     test.debug();
///     assert!(test.bark().is_err());
///
///     Ok(())
/// }
///```
/// The advantage of using views this way is that it hides the debugging/testing details from the
/// user of the class. So the developer can add in any needed test parameters to exercise the code
/// in a full and controlled way while not exposing any of those details to the user.
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[proc_macro_error]
pub fn class(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_class(attr, item, false)
}

/// This is the debug version of [`class()`]. This should only be used in testing. It displays additional
/// debugging information that may be helpful in particular for the development of this macro.
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[proc_macro_error]
pub fn debug_class(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_class(attr, item, true)
}
