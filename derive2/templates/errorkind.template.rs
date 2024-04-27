impl ErrorKind for ${NAME} {}

impl From<${NAME}> for Error {
	fn from(kind: ${NAME}) -> Error {
		let kind: Box<dyn ErrorKind> = Box::new(kind);
		Error::new(kind)
	}
}
