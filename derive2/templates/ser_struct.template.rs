// generated Serializable trait implementation
impl bmw_derive::Serializable for ${NAME} {
	// read implementation
	fn read<R>(reader: &mut R) -> Result<Self, bmw_err::Error> where R: bmw_derive::Reader {
		${RET_READ}
		${FIELD_NAME_RETURN}
	}
	// write implementation
	fn write<W>(&self, writer: &mut W) -> Result<(), bmw_err::Error> where W: bmw_derive::Writer {
		${RET_WRITE}
		Ok(())
	}
}


