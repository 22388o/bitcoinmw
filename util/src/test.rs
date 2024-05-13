use bmw_core::*;
use std::sync::{Arc, RwLock, RwLockReadGuard};

#[doc(hidden)]
struct LockClass<T>
where
	T: Send + Sync + 'static,
{
	_hidden_var_struct: LockClassVar<T>,
	_hidden_const_struct: LockClassConst,
}
#[doc(hidden)]
struct LockClassVar<T>
where
	T: Send + Sync + 'static,
{
	lock: Arc<RwLock<Option<T>>>,
}
#[doc(hidden)]
trait LockClassVarBuilder {
	fn builder(constants: &LockClassConst) -> Result<Self, Error>
	where
		Self: Sized;
}
#[derive(Configurable, Clone)]
#[doc(hidden)]
struct LockClassConst {}
#[doc(hidden)]
pub use LockClassConstOptions::*;
impl Default for LockClassConst {
	fn default() -> Self {
		Self {}
	}
}
impl<T> LockClass<T>
where
	T: Send + Sync + 'static,
{
	fn constants(&self) -> &LockClassConst {
		&self._hidden_const_struct
	}
	fn vars_mut(&mut self) -> &mut LockClassVar<T> {
		&mut self._hidden_var_struct
	}
	fn vars(&self) -> &LockClassVar<T> {
		&self._hidden_var_struct
	}
	fn builder(_hidden_const_struct: LockClassConst) -> Result<Self, Error> {
		let _hidden_var_struct = LockClassVar::builder(&_hidden_const_struct)?;
		Ok(Self {
			_hidden_var_struct,
			_hidden_const_struct,
		})
	}
}
impl<T> LockClassVar<T>
where
	T: Send + Sync + 'static,
{
	pub(crate) fn get_lock(&self) -> &Arc<RwLock<Option<T>>> {
		&self.lock
	}
	pub(crate) fn get_mut_lock(&mut self) -> &mut Arc<RwLock<Option<T>>> {
		&mut self.lock
	}
}
impl LockClassConst {}
trait Lock<T>
where
	T: Send + Sync + 'static,
{
	#[document]
	fn init(&mut self, t: T) -> Result<(), Error>;
	//#[document]
	fn rlock(&self) -> Result<RwLockReadGuard<'_, Option<T>>, Error>;
	#[doc(hidden)]
	fn configurable_mut(&mut self) -> &mut dyn Configurable;
	#[doc(hidden)]
	fn configurable(&self) -> &dyn Configurable;
}
impl<T> Lock<T> for LockClass<T>
where
	T: Send + Sync + 'static,
{
	fn init(&mut self, t: T) -> Result<(), Error> {
		if bmw_core::is_recursive() {
			panic!("Recursion detected! Perhaps LockClass::init is not implemented?");
		}
		LockClass::init(self, t)
	}
	fn rlock(&self) -> Result<RwLockReadGuard<'_, Option<T>>, Error> {
		if bmw_core::is_recursive() {
			panic!("Recursion detected! Perhaps LockClass::rlock is not implemented?");
		}
		LockClass::rlock(self)
	}
	fn configurable_mut(&mut self) -> &mut dyn Configurable {
		&mut self._hidden_const_struct
	}
	fn configurable(&self) -> &dyn Configurable {
		&self._hidden_const_struct
	}
}
impl<T> Lock<T> for &mut LockClass<T>
where
	T: Send + Sync + 'static,
{
	fn init(&mut self, t: T) -> Result<(), Error> {
		if bmw_core::is_recursive() {
			panic!("Recursion detected! Perhaps LockClass::init is not implemented?");
		}
		LockClass::init(self, t)
	}
	fn rlock(&self) -> Result<RwLockReadGuard<'_, Option<T>>, Error> {
		if bmw_core::is_recursive() {
			panic!("Recursion detected! Perhaps LockClass::rlock is not implemented?");
		}
		LockClass::rlock(self)
	}
	fn configurable_mut(&mut self) -> &mut dyn Configurable {
		&mut self._hidden_const_struct
	}
	fn configurable(&self) -> &dyn Configurable {
		&self._hidden_const_struct
	}
}
#[doc = "Builder for the `LockClass` class."]
struct LockClassBuilder {}
impl LockClassBuilder {
	#[doc = "Builds an instance of the [`Lock`]"]
	#[doc = "trait using the specified input parameters."]
	#[doc = "# Input Parameters"]
	#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
	#[doc = "|---|---|---|---|"]
	#[doc = "# Return
"]
	#[doc = "[`Result`]<impl [`Lock`], [`Error`]>
"]
	#[doc = "# Errors
"]
	#[doc = "[`CoreErrorKind::Configuration`] - If the configuration is invalid.<br/>"]
	#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
	#[doc = "# Also see"]
	#[doc = "[`Lock`]<br/>"]
	#[doc = "[`crate::lock`]"]
	#[doc = "[^1]: Multiple values allowed.
"]
	fn build_lock<T>(options: Vec<LockClassConstOptions>) -> Result<impl Lock<T>, Error>
	where
		T: Send + Sync + 'static,
	{
		let conf = configure!(LockClassConst, LockClassConstOptions, options)?;
		match LockClass::builder(conf) {
			Ok(v) => Ok(v),
			Err(e) => err!(CoreErrorKind::Builder, e.kind().to_string()),
		}
	}
	#[doc = "Builds an instance of the [`Lock`]"]
	#[doc = "trait using the specified input parameters."]
	#[doc = "# Input Parameters"]
	#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
	#[doc = "|---|---|---|---|"]
	#[doc = "# Return
"]
	#[doc = "[`Result`]<[`Box`]<dyn [`Lock`]>, [`Error`]>
"]
	#[doc = "# Errors
"]
	#[doc = "[`CoreErrorKind::Configuration`] - If the configuration is invalid.<br/>"]
	#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
	#[doc = "# Also see"]
	#[doc = "[`Lock`]<br/>"]
	#[doc = "[`crate::lock_box`]"]
	#[doc = "[^1]: Multiple values allowed.
"]
	fn build_lock_box<T>(options: Vec<LockClassConstOptions>) -> Result<Box<dyn Lock<T>>, Error>
	where
		T: Send + Sync + 'static,
	{
		let conf = configure!(LockClassConst, LockClassConstOptions, options)?;
		match LockClass::builder(conf) {
			Ok(v) => Ok(Box::new(v)),
			Err(e) => err!(CoreErrorKind::Builder, e.kind().to_string()),
		}
	}
	#[doc = "Builds an instance of the [`Lock`]"]
	#[doc = "trait using the specified input parameters."]
	#[doc = "# Input Parameters"]
	#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
	#[doc = "|---|---|---|---|"]
	#[doc = "# Return
"]
	#[doc = "[`Result`]<impl [`Lock`] + [`Send`], [`Error`]>
"]
	#[doc = "# Errors
"]
	#[doc = "[`CoreErrorKind::Configuration`] - If the configuration is invalid.<br/>"]
	#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
	#[doc = "# Also see"]
	#[doc = "[`Lock`]<br/>"]
	#[doc = "[`crate::lock_send`]"]
	#[doc = "[^1]: Multiple values allowed.
"]
	fn build_lock_send<T>(options: Vec<LockClassConstOptions>) -> Result<impl Lock<T> + Send, Error>
	where
		T: Send + Sync + 'static,
	{
		let conf = configure!(LockClassConst, LockClassConstOptions, options)?;
		match LockClass::builder(conf) {
			Ok(v) => Ok(v),
			Err(e) => err!(CoreErrorKind::Builder, e.kind().to_string()),
		}
	}
	#[doc = "Builds an instance of the [`Lock`]"]
	#[doc = "trait using the specified input parameters."]
	#[doc = "# Input Parameters"]
	#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
	#[doc = "|---|---|---|---|"]
	#[doc = "# Return
"]
	#[doc = "[`Result`]<[`Box`]<dyn [`Lock`] + [`Send`]>, [`Error`]>
"]
	#[doc = "# Errors
"]
	#[doc = "[`CoreErrorKind::Configuration`] - If the configuration is invalid.<br/>"]
	#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
	#[doc = "# Also see"]
	#[doc = "[`Lock`]<br/>"]
	#[doc = "[`crate::lock_send_box`]"]
	#[doc = "[^1]: Multiple values allowed.
"]
	fn build_lock_send_box<T>(
		options: Vec<LockClassConstOptions>,
	) -> Result<Box<dyn Lock<T> + Send>, Error>
	where
		T: Send + Sync + 'static,
	{
		let conf = configure!(LockClassConst, LockClassConstOptions, options)?;
		match LockClass::builder(conf) {
			Ok(v) => Ok(Box::new(v)),
			Err(e) => err!(CoreErrorKind::Builder, e.kind().to_string()),
		}
	}
	#[doc = "Builds an instance of the [`Lock`]"]
	#[doc = "trait using the specified input parameters."]
	#[doc = "# Input Parameters"]
	#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
	#[doc = "|---|---|---|---|"]
	#[doc = "# Return
"]
	#[doc = "[`Result`]<impl [`Lock`] + [`Send`] + [`Sync`], [`Error`]>
"]
	#[doc = "# Errors
"]
	#[doc = "[`CoreErrorKind::Configuration`] - If the configuration is invalid.<br/>"]
	#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
	#[doc = "# Also see"]
	#[doc = "[`Lock`]<br/>"]
	#[doc = "[`crate::lock_sync`]"]
	#[doc = "[^1]: Multiple values allowed.
"]
	fn build_lock_sync<T>(
		options: Vec<LockClassConstOptions>,
	) -> Result<impl Lock<T> + Send + Sync, Error>
	where
		T: Send + Sync + 'static,
	{
		let conf = configure!(LockClassConst, LockClassConstOptions, options)?;
		match LockClass::builder(conf) {
			Ok(v) => Ok(v),
			Err(e) => err!(CoreErrorKind::Builder, e.kind().to_string()),
		}
	}
	#[doc = "Builds an instance of the [`Lock`]"]
	#[doc = "trait using the specified input parameters."]
	#[doc = "# Input Parameters"]
	#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
	#[doc = "|---|---|---|---|"]
	#[doc = "# Return
"]
	#[doc = "[`Result`]<[`Box`]<dyn [`Lock`] + [`Send`] + [`Sync`]>, [`Error`]>
"]
	#[doc = "# Errors
"]
	#[doc = "[`CoreErrorKind::Configuration`] - If the configuration is invalid.<br/>"]
	#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
	#[doc = "# Also see"]
	#[doc = "[`Lock`]<br/>"]
	#[doc = "[`crate::lock_sync_box`]"]
	#[doc = "[^1]: Multiple values allowed.
"]
	fn build_lock_sync_box<T>(
		options: Vec<LockClassConstOptions>,
	) -> Result<Box<dyn Lock<T> + Send + Sync>, Error>
	where
		T: Send + Sync + 'static,
	{
		let conf = configure!(LockClassConst, LockClassConstOptions, options)?;
		match LockClass::builder(conf) {
			Ok(v) => Ok(Box::new(v)),
			Err(e) => err!(CoreErrorKind::Builder, e.kind().to_string()),
		}
	}
}
#[allow(unused_macros)]
#[doc = "Builds an instance of the [`Lock`]"]
#[doc = "trait using the specified input parameters."]
#[doc = "# Input Parameters"]
#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
#[doc = "|---|---|---|---|"]
#[doc = "# Return
"]
#[doc = "[`Result`]<impl [`Lock`], [`Error`]>
"]
#[doc = "# Errors
"]
#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
#[doc = "# Also see"]
#[doc = "[`Lock`]<br/>"]
#[doc = "[`LockClassBuilder::build_lock`]"]
#[doc = "[^1]: Multiple values allowed.
"]
macro_rules! lock
{
    ($($param:tt)*) =>
    {
        {
            match LockClassBuilder::build_lock(vec![$($param)*])
            {
                Ok(ret) => { Ok(ret) } Err(e) =>
                { err!(CoreErrorKind::Builder, e.to_string()) }
            }
        }
    };
}
#[allow(unused_macros)]
#[doc = "Builds an instance of the [`Lock`]"]
#[doc = "trait using the specified input parameters."]
#[doc = "# Input Parameters"]
#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
#[doc = "|---|---|---|---|"]
#[doc = "# Return
"]
#[doc = "[`Result`]<[`Box`]<dyn [`Lock`]>, [`Error`]>
"]
#[doc = "# Errors
"]
#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
#[doc = "# Also see"]
#[doc = "[`Lock`]<br/>"]
#[doc = "[`LockClassBuilder::build_lock_box`]"]
#[doc = "[^1]: Multiple values allowed.
"]
macro_rules! lock_box
{
    ($($param:tt)*) =>
    {
        {
            match LockClassBuilder::build_lock_box(vec![$($param)*])
            {
                Ok(ret) => { Ok(ret) } Err(e) =>
                { err!(CoreErrorKind::Builder, e.to_string()) }
            }
        }
    };
}
#[allow(unused_macros)]
#[doc = "Builds an instance of the [`Lock`]"]
#[doc = "trait using the specified input parameters."]
#[doc = "# Input Parameters"]
#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
#[doc = "|---|---|---|---|"]
#[doc = "# Return
"]
#[doc = "[`Result`]<impl [`Lock`] + [`Send`], [`Error`]>
"]
#[doc = "# Errors
"]
#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
#[doc = "# Also see"]
#[doc = "[`Lock`]<br/>"]
#[doc = "[`LockClassBuilder::build_lock_send`]"]
#[doc = "[^1]: Multiple values allowed.
"]
macro_rules! lock_send
{
    ($($param:tt)*) =>
    {
        {
            match LockClassBuilder::build_lock_send(vec![$($param)*])
            {
                Ok(ret) => { Ok(ret) } Err(e) =>
                { err!(CoreErrorKind::Builder, e.to_string()) }
            }
        }
    };
}
#[allow(unused_macros)]
#[doc = "Builds an instance of the [`Lock`]"]
#[doc = "trait using the specified input parameters."]
#[doc = "# Input Parameters"]
#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
#[doc = "|---|---|---|---|"]
#[doc = "# Return
"]
#[doc = "[`Result`]<[`Box`]<dyn [`Lock`] + [`Send`]>, [`Error`]>
"]
#[doc = "# Errors
"]
#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
#[doc = "# Also see"]
#[doc = "[`Lock`]<br/>"]
#[doc = "[`LockClassBuilder::build_lock_send_box`]"]
#[doc = "[^1]: Multiple values allowed.
"]
macro_rules! lock_send_box
{
    ($($param:tt)*) =>
    {
        {
            match LockClassBuilder::build_lock_send_box(vec![$($param)*])
            {
                Ok(ret) => { Ok(ret) } Err(e) =>
                { err!(CoreErrorKind::Builder, e.to_string()) }
            }
        }
    };
}
#[allow(unused_macros)]
#[doc = "Builds an instance of the [`Lock`]"]
#[doc = "trait using the specified input parameters."]
#[doc = "# Input Parameters"]
#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
#[doc = "|---|---|---|---|"]
#[doc = "# Return
"]
#[doc = "[`Result`]<impl [`Lock`] + [`Send`] + [`Sync`], [`Error`]>
"]
#[doc = "# Errors
"]
#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
#[doc = "# Also see"]
#[doc = "[`Lock`]<br/>"]
#[doc = "[`LockClassBuilder::build_lock_sync`]"]
#[doc = "[^1]: Multiple values allowed.
"]
macro_rules! lock_sync
{
    ($($param:tt)*) =>
    {
        {
            match LockClassBuilder::build_lock_sync(vec![$($param)*])
            {
                Ok(ret) => { Ok(ret) } Err(e) =>
                { err!(CoreErrorKind::Builder, e.to_string()) }
            }
        }
    };
}
#[allow(unused_macros)]
#[doc = "Builds an instance of the [`Lock`]"]
#[doc = "trait using the specified input parameters."]
#[doc = "# Input Parameters"]
#[doc = "| Parameter | Multi [^1] | Description | Default Value |"]
#[doc = "|---|---|---|---|"]
#[doc = "# Return
"]
#[doc = "[`Result`]<[`Box`]<dyn [`Lock`] + [`Send`] + [`Sync`]>, [`Error`]>
"]
#[doc = "# Errors
"]
#[doc = "[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind."]
#[doc = "# Also see"]
#[doc = "[`Lock`]<br/>"]
#[doc = "[`LockClassBuilder::build_lock_sync_box`]"]
#[doc = "[^1]: Multiple values allowed.
"]
macro_rules! lock_sync_box
{
    ($($param:tt)*) =>
    {
        {
            match LockClassBuilder::build_lock_sync_box(vec![$($param)*])
            {
                Ok(ret) => { Ok(ret) } Err(e) =>
                { err!(CoreErrorKind::Builder, e.to_string()) }
            }
        }
    };
}
impl<T> LockClass<T> where T: Send + Sync + 'static {}

impl<T> LockClassVarBuilder for LockClassVar<T>
where
	T: Send + Sync + 'static,
{
	fn builder(_constants: &LockClassConst) -> Result<Self, Error> {
		let lock = Arc::new(RwLock::new(None));
		Ok(Self { lock })
	}
}

impl<T> LockClass<T>
where
	T: Send + Sync + 'static,
{
	fn init(&mut self, t: T) -> Result<(), Error> {
		let v = self.vars_mut().get_mut_lock();
		let mut guard = v.write()?;
		*guard = Some(t);
		Ok(())
	}
	fn rlock(&self) -> Result<RwLockReadGuard<'_, Option<T>>, Error> {
		let v = self.vars().get_lock();
		let guard = v.read()?;
		Ok(guard)
	}
}
