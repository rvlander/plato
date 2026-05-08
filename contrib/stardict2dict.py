#!/usr/bin/env python3
"""Convert a StarDict dictionary to Plato's dictd format (.index + .dict.dz).

Usage: python3 contrib/stardict2dict.py path/to/dict.ifo
"""

import sys
import struct
import gzip
import os

BASE64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"

def encode_base64(n):
    if n == 0:
        return BASE64_CHARS[0]
    result = []
    while n:
        result.append(BASE64_CHARS[n % 64])
        n //= 64
    return ''.join(reversed(result))

def parse_ifo(ifo_path):
    info = {}
    with open(ifo_path) as f:
        for line in f:
            line = line.strip()
            if '=' in line:
                k, v = line.split('=', 1)
                info[k.strip()] = v.strip()
    return info

def parse_idx(idx_path):
    entries = []
    with open(idx_path, 'rb') as f:
        data = f.read()
    i = 0
    while i < len(data):
        end = data.index(b'\x00', i)
        word = data[i:end].decode('utf-8', errors='replace')
        offset = struct.unpack('>I', data[end+1:end+5])[0]
        size   = struct.unpack('>I', data[end+5:end+9])[0]
        entries.append((word, offset, size))
        i = end + 9
    return entries

def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} path/to/dict.ifo")
        sys.exit(1)

    ifo_path = sys.argv[1]
    base = os.path.splitext(ifo_path)[0]
    info = parse_ifo(ifo_path)

    # Read dict content (supports .dict.dz or .dict)
    dz_path = base + '.dict.dz'
    dict_path = base + '.dict'
    if os.path.exists(dz_path):
        with gzip.open(dz_path, 'rb') as f:
            content = f.read()
    elif os.path.exists(dict_path):
        with open(dict_path, 'rb') as f:
            content = f.read()
    else:
        print("No .dict or .dict.dz file found.")
        sys.exit(1)

    # Read index (supports .idx or already-renamed .index — both are binary StarDict format here)
    idx_path = base + '.idx'
    if not os.path.exists(idx_path):
        idx_path = base + '.index'
    entries = parse_idx(idx_path)
    index_out = base + '.index'  # will overwrite with text format

    short_name = info.get('bookname', os.path.basename(base))
    url = info.get('website', '')

    # Build new dict content and index
    new_dict = b''
    index_lines = []

    # Add dictd header entries
    header_entries = [
        ('00-database-short', f'     {short_name}\n'),
        ('00-database-url',   f'     {url}\n'),
        ('00-database-utf8',  '     utf8\n'),
    ]
    for hw, definition in header_entries:
        offset = len(new_dict)
        encoded = definition.encode('utf-8')
        new_dict += encoded
        index_lines.append(f"{hw}\t{encode_base64(offset)}\t{encode_base64(len(encoded))}")

    for word, src_offset, src_size in entries:
        definition = content[src_offset:src_offset + src_size].decode('utf-8', errors='replace')
        offset = len(new_dict)
        encoded = definition.encode('utf-8')
        new_dict += encoded
        index_lines.append(f"{word}\t{encode_base64(offset)}\t{encode_base64(len(encoded))}")

    # Write .index (text format)
    with open(index_out, 'w', encoding='utf-8') as f:
        f.write('\n'.join(index_lines) + '\n')

    # Write uncompressed .dict (Plato supports both .dict and .dict.dz)
    dict_out = base + '.dict'
    with open(dict_out, 'wb') as f:
        f.write(new_dict)

    # Remove old StarDict files
    for path in [ifo_path, idx_path, dz_path]:
        if os.path.exists(path) and path != dict_out and path != index_out:
            os.remove(path)

    print(f"Done: {index_out} + {dict_out}")
    print(f"  {len(entries)} entries")

if __name__ == '__main__':
    main()
