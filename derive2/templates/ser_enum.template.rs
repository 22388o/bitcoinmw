// generated Serializable trait implementation
impl bmw_derive::Serializable for ${NAME} {
	// read implementation
	fn read<R>(reader: &mut R) -> Result<Self, bmw_err::Error> where R: bmw_derive::Reader {
		Ok(match reader.read_u16()? {
			${RET_READ}_ => {
				let fmt = "unexpected type returned in reader";
				let e = bmw_err::err!(bmw_err::ErrKind::CorruptedData, fmt);
				return Err(e);
			}
		})
	}

	// write implementation
	fn write<W>(&self, writer: &mut W) -> Result<(), bmw_err::Error> where W: bmw_derive::Writer {
		match self {
			${RET_WRITE}}
		Ok(())
	}
}

