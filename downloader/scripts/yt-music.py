#!/usr/bin/env python3

from ytmusicapi import YTMusic
import difflib

yt_music = YTMusic()


def find_yt_music_url(track_name, album_name, artist_name):
    search_query = f'{track_name} {album_name} {artist_name}'
    res = yt_music.search(search_query, filter='songs')

    best_match_index = -1
    best_match_ratio = 0.0
    for index, song in enumerate(res):
        title = song['title']
        similarity_ratio = difflib.SequenceMatcher(None, track_name.lower(), title.lower()).ratio()
        if similarity_ratio > best_match_ratio:
            best_match_ratio = similarity_ratio
            best_match_index = index

    if best_match_index == -1:
        best_match_index = 0

    return f'https://music.youtube.com/watch?v={res[best_match_index]["videoId"]}'


if __name__ == '__main__':
    import sys
    if len(sys.argv) != 4:
        print('Usage: yt-music.py <track name> <album name> <artist name>')
        sys.exit(1)
    track_name, album_name, artist_name = sys.argv[1:]
    print(find_yt_music_url(track_name, album_name, artist_name))
