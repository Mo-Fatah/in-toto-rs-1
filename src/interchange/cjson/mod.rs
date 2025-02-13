use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use std::collections::BTreeMap;
use std::io::{Read, Write};

use crate::error::Error;
use crate::interchange::DataInterchange;
use crate::Result;

pub(crate) mod pretty;
pub(crate) mod shims;

pub use pretty::JsonPretty;

/// JSON data interchange.
///
/// # Schema
///
/// This doesn't use JSON Schema because that specification language is rage inducing. Here's
/// something else instead.
///
/// ## Common Entities
///
/// `NATURAL_NUMBER` is an integer in the range `[1, 2**32)`.
///
/// `EXPIRES` is an ISO-8601 date time in format `YYYY-MM-DD'T'hh:mm:ss'Z'`.
///
/// `KEY_ID` is the hex encoded value of `sha256(cjson(pub_key))`.
///
/// `PUB_KEY` is the following:
///
/// ```bash
/// {
///   "type": KEY_TYPE,
///   "scheme": SCHEME,
///   "value": PUBLIC
/// }
/// ```
///
/// `PUBLIC` is a base64url encoded `SubjectPublicKeyInfo` DER public key.
///
/// `KEY_TYPE` is a string (either `rsa` or `ed25519`).
///
/// `SCHEME` is a string (either `ed25519`, `rsassa-pss-sha256`, or `rsassa-pss-sha512`
///
/// `HASH_VALUE` is a hex encoded hash value.
///
/// `SIG_VALUE` is a hex encoded signature value.
///
/// `METADATA_DESCRIPTION` is the following:
///
/// ```bash
/// {
///   "version": NATURAL_NUMBER,
///   "length": NATURAL_NUMBER,
///   "hashes": {
///     HASH_ALGORITHM: HASH_VALUE
///     ...
///   }
/// }
/// ```
///
/// ## `Metablock`
///
/// ```bash
/// {
///   "signatures": [SIGNATURE],
///   "signed": SIGNED
/// }
/// ```
///
/// `SIGNATURE` is:
///
/// ```bash
/// {
///   "keyid": KEY_ID,
///   "signature": SIG_VALUE
/// }
/// ```
///
/// `SIGNED` is one of:
///
/// - `RootMetadata`
/// - `SnapshotMetadata`
/// - `TargetsMetadata`
/// - `TimestampMetadata`
///
/// The the elements of `signatures` must have unique `key_id`s.
///
/// ## `RootMetadata`
///
/// ```bash
/// {
///   "_type": "root",
///   "version": NATURAL_NUMBER,
///   "expires": EXPIRES,
///   "keys": [PUB_KEY, ...]
///   "roles": {
///     "root": ROLE_DESCRIPTION,
///     "snapshot": ROLE_DESCRIPTION,
///     "targets": ROLE_DESCRIPTION,
///     "timestamp": ROLE_DESCRIPTION
///   }
/// }
/// ```
///
/// `ROLE_DESCRIPTION` is the following:
///
/// ```bash
/// {
///   "threshold": NATURAL_NUMBER,
///   "keyids": [KEY_ID, ...]
/// }
/// ```
///
/// ## `SnapshotMetadata`
///
/// ```bash
/// {
///   "_type": "snapshot",
///   "version": NATURAL_NUMBER,
///   "expires": EXPIRES,
///   "meta": {
///     META_PATH: METADATA_DESCRIPTION
///   }
/// }
/// ```
///
/// `META_PATH` is a string.
///
///
/// ## `TargetsMetadata`
///
/// ```bash
/// {
///   "_type": "timestamp",
///   "version": NATURAL_NUMBER,
///   "expires": EXPIRES,
///   "targets": {
///     TARGET_PATH: TARGET_DESCRIPTION
///     ...
///   },
/// }
/// ```
///
/// `ROLE` is a string,
///
/// `PATH` is a string.
///
/// ## `TimestampMetadata`
///
/// ```bash
/// {
///   "_type": "timestamp",
///   "version": NATURAL_NUMBER,
///   "expires": EXPIRES,
///   "snapshot": METADATA_DESCRIPTION
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Json;

impl DataInterchange for Json {
    type RawData = serde_json::Value;

    /// ```
    /// # use in_toto::interchange::{DataInterchange, Json};
    /// assert_eq!(Json::extension(), "json");
    /// ```
    fn extension() -> &'static str {
        "json"
    }

    /// ```
    /// # use in_toto::interchange::{DataInterchange, Json};
    /// # use std::collections::HashMap;
    /// let jsn: &[u8] = br#"{"foo": "bar", "baz": "quux"}"#;
    /// let raw = Json::from_reader(jsn).unwrap();
    /// let out = Json::canonicalize(&raw).unwrap();
    /// assert_eq!(out, br#"{"baz":"quux","foo":"bar"}"#);
    /// ```
    fn canonicalize(raw_data: &Self::RawData) -> Result<Vec<u8>> {
        canonicalize(raw_data).map_err(Error::Opaque)
    }

    /// ```
    /// # use serde_derive::Deserialize;
    /// # use serde_json::json;
    /// # use std::collections::HashMap;
    /// # use in_toto::interchange::{DataInterchange, Json};
    /// #
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Thing {
    ///    foo: String,
    ///    bar: String,
    /// }
    ///
    /// # fn main() {
    /// let jsn = json!({"foo": "wat", "bar": "lol"});
    /// let thing = Thing { foo: "wat".into(), bar: "lol".into() };
    /// let de: Thing = Json::deserialize(&jsn).unwrap();
    /// assert_eq!(de, thing);
    /// # }
    /// ```
    fn deserialize<T>(raw_data: &Self::RawData) -> Result<T>
    where
        T: DeserializeOwned,
    {
        Ok(serde_json::from_value(raw_data.clone())?)
    }

    /// ```
    /// # use serde_derive::Serialize;
    /// # use serde_json::json;
    /// # use std::collections::HashMap;
    /// # use in_toto::interchange::{DataInterchange, Json};
    /// #
    /// #[derive(Serialize)]
    /// struct Thing {
    ///    foo: String,
    ///    bar: String,
    /// }
    ///
    /// # fn main() {
    /// let jsn = json!({"foo": "wat", "bar": "lol"});
    /// let thing = Thing { foo: "wat".into(), bar: "lol".into() };
    /// let se: serde_json::Value = Json::serialize(&thing).unwrap();
    /// assert_eq!(se, jsn);
    /// # }
    /// ```
    fn serialize<T>(data: &T) -> Result<Self::RawData>
    where
        T: Serialize,
    {
        Ok(serde_json::to_value(data)?)
    }

    /// ```
    /// # use serde_json::json;
    /// # use in_toto::interchange::{DataInterchange, Json};
    /// let json = json!({
    ///     "o": {
    ///         "a": [1, 2, 3],
    ///         "s": "string",
    ///         "n": 123,
    ///         "t": true,
    ///         "f": false,
    ///         "0": null,
    ///     },
    /// });
    /// let mut buf = Vec::new();
    /// Json::to_writer(&mut buf, &json).unwrap();
    /// assert_eq!(
    ///     &String::from_utf8(buf).unwrap(),
    ///     r#"{"o":{"0":null,"a":[1,2,3],"f":false,"n":123,"s":"string","t":true}}"#
    /// );
    /// ```
    fn to_writer<W, T: Sized>(mut writer: W, value: &T) -> Result<()>
    where
        W: Write,
        T: Serialize,
    {
        let bytes = Self::canonicalize(&Self::serialize(value)?)?;
        writer.write_all(&bytes)?;
        Ok(())
    }

    /// ```
    /// # use in_toto::interchange::{DataInterchange, Json};
    /// # use std::collections::HashMap;
    /// let jsn: &[u8] = br#"{"foo": "bar", "baz": "quux"}"#;
    /// let _: HashMap<String, String> = Json::from_reader(jsn).unwrap();
    /// ```
    fn from_reader<R, T>(rdr: R) -> Result<T>
    where
        R: Read,
        T: DeserializeOwned,
    {
        Ok(serde_json::from_reader(rdr)?)
    }

    /// ```
    /// # use in_toto::interchange::{DataInterchange, Json};
    /// # use std::collections::HashMap;
    /// let jsn: &[u8] = br#"{"foo": "bar", "baz": "quux"}"#;
    /// let _: HashMap<String, String> = Json::from_slice(&jsn).unwrap();
    /// ```
    fn from_slice<T>(slice: &[u8]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        Ok(serde_json::from_slice(slice)?)
    }
}

fn canonicalize(jsn: &serde_json::Value) -> std::result::Result<Vec<u8>, String> {
    let converted = convert(jsn)?;
    let mut buf = Vec::new();
    let _ = converted.write(&mut buf); // Vec<u8> impl always succeeds (or panics).
    Ok(buf)
}

enum Value {
    Array(Vec<Value>),
    Bool(bool),
    Null,
    Number(Number),
    Object(BTreeMap<String, Value>),
    String(String),
}

impl Value {
    fn write(&self, mut buf: &mut Vec<u8>) -> std::result::Result<(), String> {
        match *self {
            Value::Null => {
                buf.extend(b"null");
                Ok(())
            }
            Value::Bool(true) => {
                buf.extend(b"true");
                Ok(())
            }
            Value::Bool(false) => {
                buf.extend(b"false");
                Ok(())
            }
            Value::Number(Number::I64(n)) => itoa::write(buf, n)
                .map(|_| ())
                .map_err(|err| format!("Write error: {}", err)),
            Value::Number(Number::U64(n)) => itoa::write(buf, n)
                .map(|_| ())
                .map_err(|err| format!("Write error: {}", err)),
            Value::String(ref s) => {
                // this mess is abusing serde_json to get json escaping
                let s = serde_json::Value::String(s.clone());
                let s = serde_json::to_string(&s).map_err(|e| format!("{:?}", e))?;
                buf.extend(s.as_bytes());
                Ok(())
            }
            Value::Array(ref arr) => {
                buf.push(b'[');
                let mut first = true;
                for a in arr.iter() {
                    if !first {
                        buf.push(b',');
                    }
                    a.write(&mut buf)?;
                    first = false;
                }
                buf.push(b']');
                Ok(())
            }
            Value::Object(ref obj) => {
                buf.push(b'{');
                let mut first = true;
                for (k, v) in obj.iter() {
                    if !first {
                        buf.push(b',');
                    }
                    first = false;

                    // this mess is abusing serde_json to get json escaping
                    let k = serde_json::Value::String(k.clone());
                    let k = serde_json::to_string(&k).map_err(|e| format!("{:?}", e))?;
                    buf.extend(k.as_bytes());

                    buf.push(b':');
                    v.write(&mut buf)?;
                }
                buf.push(b'}');
                Ok(())
            }
        }
    }
}

enum Number {
    I64(i64),
    U64(u64),
}

fn convert(jsn: &serde_json::Value) -> std::result::Result<Value, String> {
    match *jsn {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Bool(b)),
        serde_json::Value::Number(ref n) => n
            .as_i64()
            .map(Number::I64)
            .or_else(|| n.as_u64().map(Number::U64))
            .map(Value::Number)
            .ok_or_else(|| String::from("only i64 and u64 are supported")),
        serde_json::Value::Array(ref arr) => {
            let mut out = Vec::new();
            for res in arr.iter().map(|v| convert(v)) {
                out.push(res?)
            }
            Ok(Value::Array(out))
        }
        serde_json::Value::Object(ref obj) => {
            let mut out = BTreeMap::new();
            for (k, v) in obj.iter() {
                let _ = out.insert(k.clone(), convert(v)?);
            }
            Ok(Value::Object(out))
        }
        serde_json::Value::String(ref s) => Ok(Value::String(s.clone())),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn write_str() {
        let jsn = Value::String(String::from("wat"));
        let mut out = Vec::new();
        jsn.write(&mut out).unwrap();
        assert_eq!(&out, b"\"wat\"");
    }

    #[test]
    fn write_arr() {
        let jsn = Value::Array(vec![
            Value::String(String::from("wat")),
            Value::String(String::from("lol")),
            Value::String(String::from("no")),
        ]);
        let mut out = Vec::new();
        jsn.write(&mut out).unwrap();
        assert_eq!(&out, b"[\"wat\",\"lol\",\"no\"]");
    }

    #[test]
    fn write_obj() {
        let mut map = BTreeMap::new();
        let arr = Value::Array(vec![
            Value::String(String::from("haha")),
            Value::String(String::from("new\nline")),
        ]);
        let _ = map.insert(String::from("lol"), arr);
        let jsn = Value::Object(map);
        let mut out = Vec::new();
        jsn.write(&mut out).unwrap();
        assert_eq!(&out, &b"{\"lol\":[\"haha\",\"new\\nline\"]}");
    }
}
