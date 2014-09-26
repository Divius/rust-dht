// Copyright 2014 Dmitry "Divius" Tantsur <divius.inside@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//

//! KRPC implementation as described in
//! [BEP 0005](http://www.bittorrent.org/beps/bep_0005.html).

use std::collections;

use bencode::{mod, ToBencode};
use bencode::util::ByteString;

use super::Node;


/// Mapping String -> Bytes used in payload.
pub type BDict = collections::TreeMap<String, Vec<u8>>;

/// Package payload in KRPC: either Query (request) or Response or Error.
pub enum PackagePayload {
    /// Request to a node.
    Query(BDict),
    /// Response to request.
    Response(BDict),
    /// Error: code and string message.
    Error(i64, String)
}

/// KRPC package.
pub struct Package {
    /// Transaction ID generated by requester and passed back by responder.
    pub transaction_id: Vec<u8>,
    /// Package payload.
    pub payload: PackagePayload,
    /// Sender Node (note that as per BEP 0005 it is stored in payload).
    pub sender: Node
}


impl Package {
    fn bdict_to_bencode(&self, d: &BDict) -> bencode::Bencode {
        let mut result: bencode::DictMap = collections::TreeMap::new();
        for (key, value) in d.iter() {
            result.insert(ByteString::from_str(key.as_slice()),
                          value.to_bencode());
        }
        // TODO(divius): encode sender
        bencode::Dict(result)
    }
}

// FIXME(divius): should be upstream in bencode
#[inline]
fn bytes_to_bencode(bytes: &Vec<u8>) -> bencode::Bencode {
    bencode::ByteString(bytes.clone())
}

// FIXME(divius): should be upstream in bencode
#[inline]
fn str_to_bencode(s: &str) -> bencode::Bencode {
    bytes_to_bencode(&s.as_bytes().to_vec())
}

impl ToBencode for Package {
    fn to_bencode(&self) -> bencode::Bencode {
        // FIXME(divius): could be just TreeMap<String, Bencode>
        // if Bencode type implemented ToBencode. Move upstream?
        let mut result: bencode::DictMap = collections::TreeMap::new();

        result.insert(ByteString::from_str("tt"),
                      bytes_to_bencode(&self.transaction_id));
        let (typ, payload) = match self.payload {
            Query(ref d) => ("q", self.bdict_to_bencode(d)),
            Response(ref d) => ("r", self.bdict_to_bencode(d)),
            Error(code, ref s) => {
                let l = vec![code.to_bencode(), s.to_bencode()];
                ("e", bencode::List(l))
            }
        };
        // FIXME(divius): move to upstream bencode:
        // ToBencode should be implemented for &str
        result.insert(ByteString::from_str("y"), str_to_bencode(typ));
        result.insert(ByteString::from_str(typ), payload);

        bencode::Dict(result)
    }
}


#[cfg(test)]
mod test {
    use std::collections;

    use bencode::{mod, ToBencode};

    use super::BDict;
    use super::Error;
    use super::Package;
    use super::PackagePayload;
    use super::Query;
    use super::Response;

    use super::super::utils::test;


    fn new_package(payload: PackagePayload) -> Package {
        Package {
            transaction_id: vec![1, 2, 254, 255],
            sender: test::new_node(42),
            payload: payload
        }
    }

    fn common<'a>(b: &'a bencode::Bencode, typ: &str) -> &'a bencode::DictMap {
        match *b {
            bencode::Dict(ref d) => {
                let tt_val = &d[bencode::util::ByteString::from_str("tt")];
                match *tt_val {
                    bencode::ByteString(ref v) => {
                        assert_eq!(vec![1, 2, 254, 255], *v);
                    },
                    _ => fail!("unexpected {}", tt_val)
                };

                let y_val = &d[bencode::util::ByteString::from_str("y")];
                match *y_val {
                    bencode::ByteString(ref v) => {
                        assert_eq!(typ.as_bytes(), v.as_slice());
                    },
                    _ => fail!("unexpected {}", y_val)
                };

                d
            },
            _ => fail!("unexpected {}", b)
        }
    }

    fn dict<'a>(b: &'a bencode::Bencode, typ: &str) -> &'a bencode::DictMap {
        let d = common(b, typ);

        let typ_val = &d[bencode::util::ByteString::from_str(typ)];
        match *typ_val {
            bencode::Dict(ref m) => m,
            _ => fail!("unexpected {}", typ_val)
        }
    }

    fn list<'a>(b: &'a bencode::Bencode, typ: &str) -> &'a bencode::ListVec {
        let d = common(b, typ);

        let typ_val = &d[bencode::util::ByteString::from_str(typ)];
        match *typ_val {
            bencode::List(ref l) => l,
            _ => fail!("unexpected {}", typ_val)
        }
    }

    #[test]
    fn test_error_to_bencode() {
        let p = new_package(Error(10, "error".to_string()));
        let enc = p.to_bencode();
        let l = list(&enc, "e");
        assert_eq!(vec![bencode::Number(10),
                        super::str_to_bencode("error")],
                   *l);
    }

    #[test]
    fn test_query_to_bencode() {
        let payload: BDict = collections::TreeMap::new();
        let p = new_package(Query(payload));
        let enc = p.to_bencode();
        dict(&enc, "q");
        // TODO(divius): Moar tests
    }

    #[test]
    fn test_response_to_bencode() {
        let payload: BDict = collections::TreeMap::new();
        let p = new_package(Response(payload));
        let enc = p.to_bencode();
        dict(&enc, "r");
        // TODO(divius): Moar tests
    }
}