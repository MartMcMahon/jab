use crate::client::Client;
use crate::peer::Peer;
use crate::torrent::{Torrent, TorrentFile};
use clap::Parser;
use hex;
mod bencode;
mod client;
mod peer;
mod tests;
mod torrent;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    Decode {
        value: String,
    },
    Info {
        torrent: String,
    },
    Peers {
        torrent: String,
    },
    Handshake {
        torrent: String,
        peer_string: String,
    },
    #[clap(alias = "download_piece")]
    DownloadPiece {
        #[clap(short = 'o')]
        filename: String,
        torrent: String,
        index: u32,
    },
    Download {
        #[clap(short = 'o')]
        target_filename: String,
        torrent: String,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Command::Decode { value } => {
            let decoded_value = bencode::decode_bencoded_value(value.into_bytes()).0;
            println!("{}", decoded_value.serialize());
        }
        Command::Info { torrent } => {
            let file: Vec<u8> = std::fs::read(&torrent).unwrap();
            let torrent: TorrentFile = serde_bencode::from_bytes(&file).unwrap();
            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", torrent.info.length);

            // hash
            let hash = torrent.info.hash();
            println!("Info Hash: {}", hex::encode(hash));

            println!("Piece Length: {}", torrent.info.piece_length);
            println!("Piece Hashes:");

            let mut hashes: Vec<Vec<u8>> = Vec::new();
            let mut peices = torrent.info.pieces.clone().into_vec();
            while peices.len() > 20 {
                let rest = peices.split_off(20);
                println!("{}", hex::encode(&peices));
                hashes.push(peices);
                peices = rest;
            }
            println!("{}", hex::encode(&peices));
            hashes.push(peices);
        }
        Command::Peers { torrent } => {
            let torrent: Torrent = Torrent::from_file(torrent);
            let info_hash = torrent.torrent_file.info.hash();
            let peer_ips = torrent.peer_ips().await;

            for peer in peer_ips {
                println!("{}", peer.to_string());
            }
        }

        // ./your_bittorrent.sh handshake sample.torrent <peer_ip>:<peer_port>
        Command::Handshake {
            torrent,
            peer_string,
        } => {
            let torrent: Torrent = Torrent::from_file(torrent);
            let peer_id = b"00112233445566778899".to_owned();
            let mut peer = Peer::new(peer_string).await;
            let handshake = peer.handshake(&torrent.torrent_file, peer_id).await;

            println!("Peer ID: {}", hex::encode(handshake.peer_id));
        }

        // ./your_bittorrent.sh download_piece -o /tmp/test-piece-0 sample.torrent 0
        Command::DownloadPiece {
            filename,
            torrent,
            index,
        } => {
            let mut client = Client::from_torrent_file(torrent).await;

            client.torrent.download_piece(index, filename).await;

            // let x = client.dl_loop().await.unwrap();
            // println!("{:#?}", x);
        }
        Command::Download {
            target_filename,
            torrent,
        } => {
            let mut client = Client::from_torrent_file(torrent).await;

            client.torrent.download(target_filename).await;
        }
    }
}
