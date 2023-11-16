use crate::peer::{Message, MessageId, Peer, PiecePayload, RequestPayload};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::net::{Ipv4Addr, SocketAddrV4};

const DEFAULT_BLOCK_SIZE: u32 = 1 << 14;

#[derive(Debug)]
pub enum TorrentState {
    Init,
    Downloading,
    Seeding,
    Complete,
}
#[derive(Debug)]
pub enum DownloadState {
    Zero,
    Partial,
    Complete,
}
#[derive(Debug)]
pub struct Piece {
    index: u32,
    state: DownloadState,
    n_blocks: u32,
}
#[derive(Debug)]
pub struct Block {
    index: u32,
    state: DownloadState,
    bytes: Vec<u8>,
    //[u8; DEFAULT_BLOCK_SIZE as usize],
}

#[derive(Debug, Deserialize, Serialize)]
struct PeersRequest<'a> {
    peer_id: &'a str,
    port: u32,
    uploaded: u32,
    downloaded: u32,
    left: u32,
    compact: u32,
}
#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct TorrentFile {
    pub announce: String,
    pub info: Info,
}
pub struct Torrent {
    pub torrent_file: TorrentFile,
    pub n_pieces: u32,
    pub peers: Vec<Peer>,
    pub pieces: Vec<Piece>,
}
impl Torrent {
    pub fn from_file(filename: String) -> Self {
        let file: Vec<u8> = std::fs::read(&filename).unwrap();
        let torrent_file: TorrentFile = serde_bencode::from_bytes(&file).unwrap();

        let n_pieces =
            (torrent_file.info.length as f32 / torrent_file.info.piece_length as f32).ceil() as u32;
        let mut pieces = Vec::with_capacity(n_pieces as usize);
        for index in 0..n_pieces {
            let n_blocks;
            if index == n_pieces - 1 {
                n_blocks = (((torrent_file.info.length % torrent_file.info.piece_length) as f32
                    / DEFAULT_BLOCK_SIZE as f32)
                    .ceil()) as u32;
            } else {
                n_blocks = (torrent_file.info.piece_length as f32 / DEFAULT_BLOCK_SIZE as f32)
                    .ceil() as u32;
            }

            pieces.push(Piece {
                index,
                state: DownloadState::Zero,
                n_blocks,
            });
        }

        Self {
            torrent_file,
            n_pieces,
            peers: Vec::new(), // defined for now in client::from_torrent_file.
            // should be moved to this function
            pieces,
        }
    }

    pub async fn get_peers(self: &Self) -> PeersResponse {
        let info_hash = self.torrent_file.info.hash();
        let peers_req = PeersRequest {
            peer_id: "00112233445566778899",
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: self.torrent_file.info.length,
            compact: 1,
        };
        let url = format!(
            "{}?{}&info_hash={}",
            &self.torrent_file.announce,
            serde_urlencoded::to_string(peers_req).unwrap(),
            urlencode_info_hash(&info_hash)
        );

        let x = reqwest::get(url).await.unwrap();
        let y: Vec<u8> = x.bytes().await.unwrap().to_vec();
        let z: PeersResponse = serde_bencode::from_bytes(&y.as_slice()).unwrap();
        z
    }

    pub async fn peer_ips(self: &Self) -> Vec<SocketAddrV4> {
        let peers_res = self.get_peers().await;
        let mut peers: Vec<SocketAddrV4> = Vec::new();
        for chunk in peers_res.peers.chunks_exact(6) {
            peers.push(SocketAddrV4::new(
                Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]),
                u16::from_be_bytes([chunk[4], chunk[5]]),
            ));
        }
        peers
    }

    pub async fn download_piece(self: &mut Self, piece_index: u32, filename: String) -> Vec<u8> {
        let n_blocks = self.pieces[piece_index as usize].n_blocks;
        // (self.torrent_file.info.piece_length as f32 / DEFAULT_BLOCK_SIZE as f32).ceil() as u32;
        let mut blocks: Vec<Block> = Vec::new();

        for block_index in 0..n_blocks {
            let block_message = self.download_block(piece_index, block_index).await;
            let block =
                PiecePayload::from_bytes(block_message.payload.as_ref().unwrap().to_owned());
            blocks.push(Block {
                index: block.index,
                state: DownloadState::Complete,
                bytes: block.block_bytes,
            });
        }

        println!("got the whole piece now");
        blocks.sort_by(|a, b| a.index.cmp(&b.index));

        // write to file
        let mut bytes: Vec<u8> = Vec::new();
        for block in blocks {
            bytes.extend(block.bytes);
        }
        println!("attempting write to {}", &filename);
        std::fs::write(&filename, &bytes).expect("error writing to file");
        println!("Piece {} downloaded to {}", piece_index, &filename);

        bytes
    }

    pub async fn download_block(self: &mut Self, piece_index: u32, block_index: u32) -> Message {
        let length: u32;
        if piece_index == self.n_pieces - 1
            && block_index == self.pieces[piece_index as usize].n_blocks - 1
        {
            length = (self.torrent_file.info.length % self.torrent_file.info.piece_length)
                % DEFAULT_BLOCK_SIZE;
        } else {
            length = DEFAULT_BLOCK_SIZE;
        }

        // send request message
        let payload = RequestPayload {
            index: piece_index,
            begin: block_index * DEFAULT_BLOCK_SIZE,
            length: length.to_be_bytes(),
        };
        let request_msg = Message::new_request_message(payload);
        // TODO: check that peer is handshook
        self.peers[0].send(request_msg.into()).await.unwrap();

        // get piece_msg
        let msg = self.peers[0].wait_for_msg(MessageId::Piece).await.unwrap();

        // println!("got message: {:#?}", msg.message_id);
        // println!("length: {:#?}", msg.length);
        // println!("block: {:#?}/ {}", block_index, n_blocks);

        msg
    }

    pub async fn download(self: &mut Self, target_filename: String) {
        let mut data: Vec<u8> = Vec::new();
        for piece_index in 0..self.n_pieces {
            let piece_bytes = self
                .download_piece(piece_index, format!("{}-{}", target_filename, piece_index))
                .await;
            // TODO check hash of piece file

            data.extend(piece_bytes);
        }

        std::fs::write(&target_filename, data)
            .expect(&format!("error writing to {}", target_filename));
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct Info {
    pub length: u32,
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u32,
    pub pieces: serde_bytes::ByteBuf,
}
impl Info {
    pub fn hash(&self) -> [u8; 20] {
        let bytes = serde_bencode::to_bytes(&self).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        let hash: [u8; 20] = hasher.finalize().try_into().unwrap();
        hash
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PeersResponse {
    // An integer, indicating how often your client should make a request to the tracker.
    interval: u32,

    // A string, which contains list of peers that your client can connect to.
    // Each peer is represented using 6 bytes. The first 4 bytes are the peer's IP address and the last 2 bytes are the peer's port number.
    pub peers: serde_bytes::ByteBuf,
}
#[allow(dead_code)]
impl PeersResponse {
    pub fn from_bytes(bytes: Bytes) -> Self {
        let x = bytes.chunks_exact(6);
        for chunk in x {
            println!("chunk {:#?}", chunk);
        }
        // let decoded = serde_bencode::from_bytes(x).unwrap();
        PeersResponse {
            interval: 0,
            peers: serde_bytes::ByteBuf::new(),
        }
    }
}

fn urlencode_info_hash(hash: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(3 * hash.len());
    for &byte in hash {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}
