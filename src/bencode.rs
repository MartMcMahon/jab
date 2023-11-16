use serde_bytes;
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BencodeValue {
    List(Vec<BencodeValue>),
    Map(HashMap<serde_bytes::ByteBuf, BencodeValue>),
    String(serde_bytes::ByteBuf),
    Int(i64),
}

impl BencodeValue {
    pub fn serialize(&self) -> serde_json::Value {
        match self {
            BencodeValue::Int(i) => {
                json!(i)
            }
            BencodeValue::String(s) => {
                json!(String::from_utf8_lossy(s).into_owned())
            }
            BencodeValue::List(l) => l.iter().map(|item| item.serialize()).collect(),
            BencodeValue::Map(m) => {
                let mut h = HashMap::new();
                for (k, v) in m.iter() {
                    h.insert(String::from_utf8_lossy(k).into_owned(), v.serialize());
                }
                json!(h)
            }
        }
    }
}

#[allow(dead_code)]
pub fn decode_bencoded_value(encoded_value: Vec<u8>) -> (BencodeValue, usize) {
    let c = encoded_value.iter().next().unwrap().to_owned() as char;
    match (c, encoded_value.clone()) {
        ('d', encoded) => {
            let mut idx = 1;
            let mut vals = HashMap::new();
            while let Some(next_byte) = encoded[idx..].iter().next() {
                let c = next_byte.to_owned() as char;
                if c == 'e' {
                    idx += 1;
                } else {
                    // let (BencodeValue::String(key_serde_val), key_len) =
                    let (key_serde_val, key_len) = decode_bencoded_value(encoded[idx..].to_vec());
                    let (val, val_len) = decode_bencoded_value(encoded[idx + key_len..].to_vec());
                    match key_serde_val {
                        BencodeValue::String(s) => {
                            vals.insert(s, val);
                        }
                        _ => {
                            panic!("dict keys must be byte strings")
                        }
                    }
                    idx += key_len + val_len;
                }
            }
            (BencodeValue::Map(vals), idx)
        }
        ('l', encoded) => {
            let mut idx = 1;
            let mut vals = Vec::new();
            while let Some(next_byte) = encoded[idx..].iter().next() {
                let c = next_byte.to_owned() as char;
                if c == 'e' {
                    idx += 1;
                    return (BencodeValue::List(vals), idx);
                } else {
                    // println!("calling recursive");
                    let (val, i) = decode_bencoded_value(encoded[idx..].to_vec());
                    vals.push(val);
                    idx += i;
                }
            }
            (BencodeValue::List(vals), idx)
        }
        ('i', encoded) => {
            let mut step = 1;
            let mut end_idx = 0;
            let mut num_bytes = String::new();
            while step < encoded.len() {
                if encoded[step] == 'e' as u8 {
                    end_idx = step;
                    break;
                } else {
                    num_bytes.push(encoded[step] as char);
                }
                step += 1;
            }

            (
                BencodeValue::Int(num_bytes.parse::<i64>().unwrap()),
                end_idx + 1,
            )
        }
        ('0'..='9', encoded) => {
            let mut step = 0;
            let mut colon_index = 0;
            let mut num_bytes = String::new();
            while step < encoded.len() {
                if encoded[step] == ':' as u8 {
                    colon_index = step;
                    break;
                } else {
                    num_bytes.push(encoded[step] as char);
                }
                step += 1;
            }
            let number = num_bytes.parse::<i64>().unwrap();
            let string = &encoded[colon_index + 1..colon_index + 1 + number as usize];
            let idx = colon_index + number as usize + 1;
            (
                BencodeValue::String(serde_bytes::ByteBuf::from(string.to_owned())),
                idx,
            )
        }
        _ => {
            panic!("Unhandled encoded value: {:#?}", encoded_value.clone())
        }
    }
}

#[allow(dead_code)]
pub fn debencode(val: String) -> serde_json::Value {
    let v = decode_bencoded_value(val.into_bytes()).0;
    v.serialize()
}
