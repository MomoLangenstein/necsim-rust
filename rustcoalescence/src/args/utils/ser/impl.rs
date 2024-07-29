use serde::{
    ser::{
        Error, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Serialize, Serializer,
};

pub struct BufferingSerializer;

#[derive(Clone, Debug)]
pub enum BufferingSerialize {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    F32(f32),
    F64(f64),
    Char(char),
    Str(Box<str>),
    Bytes(Box<[u8]>),
    None,
    Some(Box<BufferingSerialize>),
    Unit,
    UnitStruct {
        name: &'static str,
    },
    UnitVariant {
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    },
    NewtypeStruct {
        name: &'static str,
        value: Box<BufferingSerialize>,
    },
    NewtypeVariant {
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: Box<BufferingSerialize>,
    },
    Seq {
        len: Option<usize>,
        elements: Box<[BufferingSerialize]>,
    },
    Tuple {
        len: usize,
        fields: Box<[BufferingSerialize]>,
    },
    TupleStruct {
        name: &'static str,
        len: usize,
        fields: Box<[BufferingSerialize]>,
    },
    TupleVariant {
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
        fields: Box<[BufferingSerialize]>,
    },
    Map {
        len: Option<usize>,
        entries: Box<[(Option<BufferingSerialize>, Option<BufferingSerialize>)]>,
    },
    Struct {
        name: &'static str,
        len: usize,
        fields: Box<[(&'static str, Option<BufferingSerialize>)]>,
    },
    StructVariant {
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
        fields: Box<[(&'static str, Option<BufferingSerialize>)]>,
    },
}

#[derive(Clone, Debug)]
pub struct BufferingError(Box<str>);

impl std::fmt::Display for BufferingError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str(&self.0)
    }
}

impl std::error::Error for BufferingError {}

impl Error for BufferingError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        BufferingError(msg.to_string().into_boxed_str())
    }
}

pub struct BufferedSequenceSerializer {
    len: Option<usize>,
    elements: Vec<BufferingSerialize>,
}

impl SerializeSeq for BufferedSequenceSerializer {
    type Error = BufferingError;
    type Ok = BufferingSerialize;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.elements.push(value.serialize(BufferingSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Seq {
            len: self.len,
            elements: self.elements.into_boxed_slice(),
        })
    }
}

pub struct BufferedTupleSerializer {
    len: usize,
    fields: Vec<BufferingSerialize>,
}

impl SerializeTuple for BufferedTupleSerializer {
    type Error = BufferingError;
    type Ok = BufferingSerialize;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.fields.push(value.serialize(BufferingSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Tuple {
            len: self.len,
            fields: self.fields.into_boxed_slice(),
        })
    }
}

pub struct BufferedTupleStructSerializer {
    name: &'static str,
    len: usize,
    fields: Vec<BufferingSerialize>,
}

impl SerializeTupleStruct for BufferedTupleStructSerializer {
    type Error = BufferingError;
    type Ok = BufferingSerialize;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.fields.push(value.serialize(BufferingSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::TupleStruct {
            name: self.name,
            len: self.len,
            fields: self.fields.into_boxed_slice(),
        })
    }
}

pub struct BufferedTupleVariantSerializer {
    name: &'static str,
    variant_index: u32,
    variant: &'static str,
    len: usize,
    fields: Vec<BufferingSerialize>,
}

impl SerializeTupleVariant for BufferedTupleVariantSerializer {
    type Error = BufferingError;
    type Ok = BufferingSerialize;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.fields.push(value.serialize(BufferingSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::TupleVariant {
            name: self.name,
            variant_index: self.variant_index,
            variant: self.variant,
            len: self.len,
            fields: self.fields.into_boxed_slice(),
        })
    }
}

pub struct BufferedMapSerializer {
    len: Option<usize>,
    entries: Vec<(Option<BufferingSerialize>, Option<BufferingSerialize>)>,
}

impl SerializeMap for BufferedMapSerializer {
    type Error = BufferingError;
    type Ok = BufferingSerialize;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.entries
            .push((Some(key.serialize(BufferingSerializer)?), None));
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.entries
            .push((None, Some(value.serialize(BufferingSerializer)?)));
        Ok(())
    }

    fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error> {
        self.entries.push((
            Some(key.serialize(BufferingSerializer)?),
            Some(value.serialize(BufferingSerializer)?),
        ));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Map {
            len: self.len,
            entries: self.entries.into_boxed_slice(),
        })
    }
}

pub struct BufferedStructSerializer {
    name: &'static str,
    len: usize,
    fields: Vec<(&'static str, Option<BufferingSerialize>)>,
}

impl SerializeStruct for BufferedStructSerializer {
    type Error = BufferingError;
    type Ok = BufferingSerialize;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.fields
            .push((key, Some(value.serialize(BufferingSerializer)?)));
        Ok(())
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
        self.fields.push((key, None));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Struct {
            name: self.name,
            len: self.len,
            fields: self.fields.into_boxed_slice(),
        })
    }
}

pub struct BufferedStructVariantSerializer {
    name: &'static str,
    variant_index: u32,
    variant: &'static str,
    len: usize,
    fields: Vec<(&'static str, Option<BufferingSerialize>)>,
}

impl SerializeStructVariant for BufferedStructVariantSerializer {
    type Error = BufferingError;
    type Ok = BufferingSerialize;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.fields
            .push((key, Some(value.serialize(BufferingSerializer)?)));
        Ok(())
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
        self.fields.push((key, None));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::StructVariant {
            name: self.name,
            variant_index: self.variant_index,
            variant: self.variant,
            len: self.len,
            fields: self.fields.into_boxed_slice(),
        })
    }
}

impl Serializer for BufferingSerializer {
    type Error = BufferingError;
    type Ok = BufferingSerialize;
    type SerializeMap = BufferedMapSerializer;
    type SerializeSeq = BufferedSequenceSerializer;
    type SerializeStruct = BufferedStructSerializer;
    type SerializeStructVariant = BufferedStructVariantSerializer;
    type SerializeTuple = BufferedTupleSerializer;
    type SerializeTupleStruct = BufferedTupleStructSerializer;
    type SerializeTupleVariant = BufferedTupleVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::I8(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::I16(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::I32(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::I64(v))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::I128(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::U8(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::U16(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::U32(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::U64(v))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::U128(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::F32(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::F64(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Char(v))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Str(v.to_owned().into_boxed_str()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Bytes(v.to_owned().into_boxed_slice()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::None)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Some(Box::new(value.serialize(self)?)))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::Unit)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::UnitStruct { name })
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::UnitVariant {
            name,
            variant_index,
            variant,
        })
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::NewtypeStruct {
            name,
            value: Box::new(value.serialize(self)?),
        })
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(BufferingSerialize::NewtypeVariant {
            name,
            variant_index,
            variant,
            value: Box::new(value.serialize(self)?),
        })
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(BufferedSequenceSerializer {
            len,
            elements: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(BufferedTupleSerializer {
            len,
            fields: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(BufferedTupleStructSerializer {
            name,
            len,
            fields: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(BufferedTupleVariantSerializer {
            name,
            variant_index,
            variant,
            len,
            fields: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(BufferedMapSerializer {
            len,
            entries: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(BufferedStructSerializer {
            name,
            len,
            fields: Vec::with_capacity(len),
        })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(BufferedStructVariantSerializer {
            name,
            variant_index,
            variant,
            len,
            fields: Vec::with_capacity(len),
        })
    }
}

impl Serialize for BufferingSerialize {
    #[allow(clippy::too_many_lines)]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Bool(v) => serializer.serialize_bool(*v),
            Self::I8(v) => serializer.serialize_i8(*v),
            Self::I16(v) => serializer.serialize_i16(*v),
            Self::I32(v) => serializer.serialize_i32(*v),
            Self::I64(v) => serializer.serialize_i64(*v),
            Self::I128(v) => serializer.serialize_i128(*v),
            Self::U8(v) => serializer.serialize_u8(*v),
            Self::U16(v) => serializer.serialize_u16(*v),
            Self::U32(v) => serializer.serialize_u32(*v),
            Self::U64(v) => serializer.serialize_u64(*v),
            Self::U128(v) => serializer.serialize_u128(*v),
            Self::F32(v) => serializer.serialize_f32(*v),
            Self::F64(v) => serializer.serialize_f64(*v),
            Self::Char(v) => serializer.serialize_char(*v),
            Self::Str(v) => serializer.serialize_str(v),
            Self::Bytes(v) => serializer.serialize_bytes(v),
            Self::None => serializer.serialize_none(),
            Self::Some(value) => serializer.serialize_some(value),
            Self::Unit => serializer.serialize_unit(),
            Self::UnitStruct { name } => serializer.serialize_unit_struct(name),
            Self::UnitVariant {
                name,
                variant_index,
                variant,
            } => serializer.serialize_unit_variant(name, *variant_index, variant),
            Self::NewtypeStruct { name, value } => serializer.serialize_newtype_struct(name, value),
            Self::NewtypeVariant {
                name,
                variant_index,
                variant,
                value,
            } => serializer.serialize_newtype_variant(name, *variant_index, variant, value),
            Self::Seq { len, elements } => {
                let mut seq = serializer.serialize_seq(*len)?;

                for element in elements {
                    seq.serialize_element(element)?;
                }

                seq.end()
            },
            Self::Tuple { len, fields } => {
                let mut tuple = serializer.serialize_tuple(*len)?;

                for field in fields {
                    tuple.serialize_element(field)?;
                }

                tuple.end()
            },
            Self::TupleStruct { name, len, fields } => {
                let mut tuple_struct = serializer.serialize_tuple_struct(name, *len)?;

                for field in fields {
                    tuple_struct.serialize_field(field)?;
                }

                tuple_struct.end()
            },
            Self::TupleVariant {
                name,
                variant_index,
                variant,
                len,
                fields,
            } => {
                let mut tuple_variant =
                    serializer.serialize_tuple_variant(name, *variant_index, variant, *len)?;

                for field in fields {
                    tuple_variant.serialize_field(field)?;
                }

                tuple_variant.end()
            },
            Self::Map { len, entries } => {
                let mut map = serializer.serialize_map(*len)?;

                for (key, value) in entries {
                    match (key, value) {
                        (None, None) => (),
                        (Some(key), None) => map.serialize_key(key)?,
                        (None, Some(value)) => map.serialize_value(value)?,
                        (Some(key), Some(value)) => map.serialize_entry(key, value)?,
                    };
                }

                map.end()
            },
            Self::Struct { name, len, fields } => {
                let mut r#struct = serializer.serialize_struct(name, *len)?;

                for (key, value) in fields {
                    if let Some(value) = value {
                        r#struct.serialize_field(key, value)?;
                    } else {
                        r#struct.skip_field(key)?;
                    }
                }

                r#struct.end()
            },
            Self::StructVariant {
                name,
                variant_index,
                variant,
                len,
                fields,
            } => {
                let mut struct_variant =
                    serializer.serialize_struct_variant(name, *variant_index, variant, *len)?;

                for (key, value) in fields {
                    if let Some(value) = value {
                        struct_variant.serialize_field(key, value)?;
                    } else {
                        struct_variant.skip_field(key)?;
                    }
                }

                struct_variant.end()
            },
        }
    }
}
