use crate::torrent::TorrentFile;
use anyhow::{Ok, Result};
use serde::Serialize;
use std::mem::transmute;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, Interest},
    net::TcpStream,
};

#[repr(C)]
pub struct Handshake {
    length: u8,
    bittorrent: [u8; 19],
    reserved: [u8; 8],
    info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

pub struct Peer {
    connection: TcpStream,
}
impl Peer {
    pub async fn new(peer_string: String) -> Self {
        let connection = TcpStream::connect(peer_string).await.unwrap();
        Self { connection }
    }

    pub async fn is_ready(self: &Self) -> bool {
        loop {
            let ready = self.connection.ready(Interest::READABLE).await;
            match ready {
                Result::Ok(_ready) => {
                    return true;
                }
                _ => {}
            }
        }
    }

    pub async fn handshake(&mut self, torrent: &TorrentFile, peer_id: [u8; 20]) -> Handshake {
        let info_hash = torrent.info.hash();
        // let mut connection = TcpStream::connect(peer_string).await.unwrap();

        let mut handshake = Handshake {
            length: 19,
            bittorrent: b"BitTorrent protocol".to_owned(),
            reserved: [0; 8],
            info_hash,
            peer_id,
        };

        /*** pretty much all of this fancy memory work is from *
         * Jon Gjengset's stream of the same challenge        **/
        let handshake_bytes =
            &mut handshake as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];
        let handshake_bytes: &mut [u8; std::mem::size_of::<Handshake>()] =
            unsafe { &mut *handshake_bytes };
        self.connection.write_all(handshake_bytes).await.unwrap();
        self.connection.read_exact(handshake_bytes).await.unwrap();
        assert_eq!(handshake.bittorrent, *b"BitTorrent protocol");
        assert_eq!(handshake.reserved.len(), 8);
        assert_eq!(handshake.info_hash, info_hash);

        handshake
    }

    pub async fn wait_for_msg(&mut self, id: MessageId) -> anyhow::Result<Message> {
        loop {
            // std::thread::sleep(Duration::from_millis(1000));
            let mut msg_length: [u8; 4] = [0; 4];
            self.connection
                .read_exact(&mut msg_length)
                .await
                .expect("some length, really");

            // println!("waiting, read {:#?}", msg_length.to_vec());

            let l = u32::from_be_bytes(msg_length);
            // let l = 0;
            if l == 0 {
                println!("hb");
                return Ok(Message {
                    length: 0,
                    message_id: MessageId::Heartbeat,
                    payload: None,
                });
            }

            let mut msg: Message = Message::heartbeat();
            // let mut payload: Option<Vec<u8>>;
            let mut message_id: [u8; 1] = [0];
            self.connection
                .read_exact(&mut message_id)
                .await
                .expect("expected a message");
            if MessageId::from(message_id[0]) == id {
                println!("got {:?} as expected", id);
                msg = Message {
                    length: l,
                    message_id: id,
                    payload: None,
                };
            } else {
                println!("expected {:?} message!", id);
                println!("got: {:#?}", message_id);
            }

            if l == 1 {
                println!("no payload");
                // let mut buf: [u8; 1] = [0];
                // self.connection.read_exact(&mut buf).await.unwrap();
                // println!("\"empty\" payload: {:#?}", buf);
                msg.payload = None;
            } else {
                let mut buf: Vec<u8> = Vec::with_capacity((l - 1) as usize);
                unsafe {
                    buf.set_len(l as usize - 1);
                }
                self.connection.read_exact(&mut buf).await.unwrap();
                msg.payload = Some(buf);
            }

            return Ok(msg);
        }
    }

    pub async fn send(&mut self, buf: Vec<u8>) -> Result<()> {
        self.connection.write_all(&buf.as_slice()).await.unwrap();
        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum MessageId {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
    Heartbeat = 9,
}
impl From<u8> for MessageId {
    fn from(value: u8) -> Self {
        // if value < 9 {
        unsafe { transmute(value) }
        // } else {
        //     panic!("invalid message id")
        // }
    }
}
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Message {
    pub length: u32,
    pub message_id: MessageId,
    pub payload: Option<Vec<u8>>,
}
impl Message {
    pub fn from_bytes(buf: Vec<u8>) -> Self {
        let (left, right) = buf.split_at(4);
        let length = u32::from_be_bytes(left.try_into().unwrap());
        let (message_id, payload_bytes): (&u8, &[u8]) = right.split_first().unwrap();
        let message_id = unsafe { transmute(*message_id) };
        Message {
            length,
            message_id,
            payload: match payload_bytes.len() > 0 {
                true => Some(payload_bytes.to_vec()),
                false => None,
            },
        }
    }

    pub fn new_request_message(payload: RequestPayload) -> Message {
        let payload_vec: Vec<u8> = payload.into();
        let payload_bytes: &[u8] = payload_vec.as_slice();
        let length = std::mem::size_of::<MessageId>() + payload_bytes.len();
        // std::mem::size_of::<RequestPayload>();
        // println!("length is {}", length as u32);
        Message {
            length: length as u32,
            message_id: MessageId::Request,
            payload: Some(payload_bytes.to_vec()),
        }
    }

    pub fn heartbeat() -> Message {
        Message {
            length: 0,
            message_id: MessageId::Heartbeat,
            payload: None,
        }
    }
}
impl Into<Vec<u8>> for Message {
    fn into(self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&self.length.to_be_bytes());
        v.push(self.message_id as u8);
        if let Some(payload) = self.payload {
            v.extend_from_slice(&payload);
        }
        // println!("converting {:?} message into: {:#?}", self.message_id, v);
        v
    }
}

#[repr(C)]
#[derive(Debug, Serialize)]
pub struct RequestPayload {
    pub index: u32, // the zero-based piece index
    pub begin: u32, // the zero-based byte offset within the piece
    pub length: [u8; 4],
}
impl Into<Vec<u8>> for RequestPayload {
    fn into(self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&self.index.to_be_bytes());
        v.extend_from_slice(&self.begin.to_be_bytes());
        v.extend_from_slice(&self.length);
        v
    }
}
impl RequestPayload {
    fn as_bytes(self: Self) -> [u8; 12] {
        let mut b: [u8; 12] = [0; 12];
        let payload_vec: Vec<u8> = self.into();
        for (i, v) in payload_vec.iter().enumerate() {
            b[i] = *v;
        }
        b
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PiecePayload {
    pub index: u32, // the zero-based piece index
    pub begin: u32, // the zero-based byte offset within the piece
    pub block_bytes: Vec<u8>,
}
impl PiecePayload {
    pub fn from_bytes(buf: Vec<u8>) -> Self {
        // println!("decoding piecepayload bytes {:#?}", buf);

        let (index, rest) = buf.split_at(4);
        let (begin, block) = rest.split_at(4);
        // println!("index: {}", u32::from_be_bytes(index.try_into().unwrap()));
        // println!("begin: {}", u32::from_be_bytes(begin.try_into().unwrap()));
        // println!("block: {:#?}", &block.len());

        Self {
            index: u32::from_be_bytes([index[0], index[1], index[2], index[3]]),
            begin: u32::from_be_bytes([begin[0], begin[1], begin[2], begin[3]]),
            block_bytes: block.to_vec(),
        }
    }
}
