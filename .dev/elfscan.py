import struct, glob, os
for p in sorted(glob.glob('C:/Users/AjamX/AppData/Local/Temp/pb-beta7/podbox-*')):
    base = os.path.basename(p)
    if '.' in base:
        continue
    with open(p, 'rb') as f:
        d = f.read()
    offs = []
    i = 0
    magic = bytes([0x7f]) + b'ELF'
    while True:
        i = d.find(magic, i)
        if i < 0:
            break
        offs.append(i)
        i += 1
    for o in offs:
        if o + 20 > len(d):
            continue
        em = struct.unpack_from('<H', d, o + 18)[0]
        if em != 0x3e:
            continue
        etype, ever = struct.unpack_from('<HH', d, o + 16)
        ei = d[o:o + 16]
        print('%s off=%d e_type=%d e_version=%d ei_class=%d ei_data=%d ei_osabi=%d' % (
            base, o, etype, ever, ei[4], ei[5], ei[7]))
