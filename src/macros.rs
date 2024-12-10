#[macro_export]
macro_rules! serde_enum_as_u8 {
    ($enum_name:ident) => {
        // https://doc.rust-lang.org/reference/items/enumerations.html?search=#pointer-casting
        impl $enum_name {
            fn discriminant(&self) -> u8 {
                // This is safe if the enum only contains primitive types
                let pointer = self as *const Self as *const u8;
                unsafe { *pointer }
            }
        }

        impl Serialize for $enum_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match self {
                    Self::Custom(custom) => *custom,
                    known => known.discriminant(),
                }
                .serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $enum_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                let value = u8::deserialize(deserializer)?;

                // This assumes that Custom is the last variant of the enum
                let variant = if value < Self::Custom(0).discriminant() {
                    // The value corresponds to the discriminant of the enum
                    let result = unsafe { *(&value as *const u8 as *const Self) };
                    assert_eq!(result.discriminant(), value);

                    result
                } else {
                    Self::Custom(value)
                };

                Ok(variant)
            }
        }
    };
}
