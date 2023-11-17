This was started as the challenge on codecrafters.io. BEFORE CEO, Sarup Banskota started advertising for the website in commits to open source github repos. That kinda thing is extremely cringe.

## Usage
`jab download -o target_filename torrent_file`
Download a single file torrent

*does not yet support torrents with multiple files*


`jab -o download_piece target_filename torrent_file 0`
Download a specific piece of the file. 0 in this case.
If you provide an index greater than n_pieces, jab will probably panic.


`jab info torrent_file`
Get the info of the torrent_file. Does not download anything.

```
Tracker URL: http://bittorrent-tracker.example.com/announce
Length: 69420
Info Hash: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
Piece Length: 32768
Piece Hashes:
0123456789012345678901234567890123456789
1234567890123456789012345678901234567890
2345678901234567890123456789012345678901
3456789012345678901234567890123456789012
```
