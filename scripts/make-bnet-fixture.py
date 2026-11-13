"""Génère tests/fixtures/battlenet_product.db : un product.db d'exemple, encodé en protobuf à la main
(indépendamment de prost) d'après la structure observée sur un vrai fichier."""
import os


def varint(n):
    out = b""
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out += bytes([b | 0x80])
        else:
            return out + bytes([b])


def field_bytes(num, data):
    if isinstance(data, str):
        data = data.encode("utf-8")
    return varint(num << 3 | 2) + varint(len(data)) + data


def field_varint(num, value):
    return varint(num << 3) + varint(value)


def product(uid, code, path, installed=1, playable=1, family=None):
    settings = field_bytes(1, path) + field_bytes(2, "eu") + field_varint(3, 2) + field_bytes(6, "frFR")
    base = field_varint(1, installed) + field_varint(2, playable) + field_varint(3, 1) + field_bytes(7, "1.0.0.1")
    cached = field_bytes(1, base) + field_bytes(2, b"\x10\x00")
    body = field_bytes(1, uid) + field_bytes(2, code) + field_bytes(3, settings) + field_bytes(4, cached)
    if family:
        body += field_bytes(6, family)
    body += field_varint(7, 0) + field_bytes(8, '{"user_install_package_settings":[]}')
    return field_bytes(1, body)


db = b"".join([
    product("agent", "agent", "C:/ProgramData/Battle.net/Agent"),
    product("battle.net", "bna", "C:/Program Files (x86)/Battle.net"),
    product("wow", "wow", "D:/Jeux/World of Warcraft", family="wow"),
    product("pro", "pro", "D:/Jeux/Overwatch"),
    product("wowf", "wowf", "E:/WoW Forever", family="wow"),
    product("fenris", "fenris", "D:/Jeux/Diablo IV", installed=0, playable=0),
    product("newgame", "newgame", "D:/Jeux/Nouveau Jeu"),
])
# Champs de premier niveau sans rapport avec les installations (présents dans les vrais fichiers).
db += field_bytes(5, b"\x08\x01") + field_bytes(7, b"frFR")

out = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "tests", "fixtures", "battlenet_product.db")
with open(out, "wb") as f:
    f.write(db)
print(len(db), "octets")
