#!/usr/bin/env python3

from ytmusicapi import YTMusic

yt_music = YTMusic()

def find_yt_music_url(track_name, album_name, artist_name):
    res = yt_music.search(track_name + ' ' + album_name + ' ' + artist_name, filter='songs')
    return f'https://music.youtube.com/watch?v={res[0]["videoId"]}'

if __name__ == '__main__':
    import sys
    if len(sys.argv) != 4:
        print('Usage: yt-music.py <track name> <album name> <artist name>')
        sys.exit(1)
    track_name, album_name, artist_name = sys.argv[1:]
    print(find_yt_music_url(track_name, album_name, artist_name))
