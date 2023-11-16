use crate::peer::{Message, MessageId, Peer};
use crate::torrent::{Torrent, TorrentState};

pub struct Client {
    pub torrent: Torrent,
    pub state: TorrentState,
}
impl Client {
    pub async fn from_torrent_file(filename: String) -> Self {
        let mut torrent: Torrent = Torrent::from_file(filename);
        let peer_id = b"00112233445566778899".to_owned();

        let peers = &torrent.peer_ips().await;
        let mut peers = vec![Peer::new(peers[0].to_string()).await];

        // do handshake
        let peer = &mut peers[0];
        let _handshake = peer.handshake(&torrent.torrent_file, peer_id).await;

        // rec bitfield
        let bf_msg = peer.wait_for_msg(MessageId::Bitfield).await.unwrap();
        println!("bitfield payload: {:#?}", bf_msg.payload);

        // send interested
        let interested = Message {
            length: 1,
            message_id: MessageId::Interested,
            payload: None,
        };
        let _x = peer.send(interested.into()).await.unwrap();

        // rec unchoke
        let _msg = peer.wait_for_msg(MessageId::Unchoke).await.unwrap();

        torrent.peers = peers;

        Client {
            torrent,
            state: TorrentState::Init,
        }
    }
}
