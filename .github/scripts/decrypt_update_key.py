import base64
import hashlib
import os
import sys

KDF_NONE = b"\0\0"
KDF_SCRYPT = b"Sc"
HEADER_LEN = 2 + 2 + 2 + 32 + 8 + 8
KEYNUM_LEN = 8
SECRET_LEN = 64
CHECKSUM_LEN = 32
KEY_LEN = HEADER_LEN + KEYNUM_LEN + SECRET_LEN + CHECKSUM_LEN
N_LOG2_MAX = 20


def scrypt_params(memlimit: int, opslimit: int) -> tuple[int, int, int]:
    opslimit = max(32768, opslimit)
    r = 8
    n_log2 = 1
    if opslimit < memlimit // 32:
        p = 1
        maxn = opslimit // (r * 4)
    else:
        maxn = memlimit // (r * 128)
    while n_log2 < 63 and (1 << n_log2) <= maxn // 2:
        n_log2 += 1
    if opslimit >= memlimit // 32:
        p = min(0x3FFFFFFF, (opslimit // 4) // (1 << n_log2)) // r
    if n_log2 > N_LOG2_MAX:
        sys.exit("scrypt parameters too high")
    return 1 << n_log2, r, p


def decrypt(key: bytearray, password: str) -> None:
    salt = bytes(key[6:38])
    opslimit = int.from_bytes(key[38:46], "little")
    memlimit = int.from_bytes(key[46:54], "little")
    n, r, p = scrypt_params(memlimit, opslimit)
    stream = hashlib.scrypt(
        password.encode(),
        salt=salt,
        n=n,
        r=r,
        p=p,
        maxmem=256 * n * r + (1 << 20),
        dklen=KEYNUM_LEN + SECRET_LEN + CHECKSUM_LEN,
    )
    for i, byte in enumerate(stream):
        key[HEADER_LEN + i] ^= byte


def checksum_matches(key: bytearray) -> bool:
    digest = hashlib.blake2b(digest_size=CHECKSUM_LEN)
    digest.update(key[0:2])
    digest.update(key[HEADER_LEN : HEADER_LEN + KEYNUM_LEN + SECRET_LEN])
    return digest.digest() == bytes(key[HEADER_LEN + KEYNUM_LEN + SECRET_LEN :])


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(f"usage: {sys.argv[0]} OUTPUT")

    wrapped = os.environ.get("TAURI_SIGNING_PRIVATE_KEY", "").strip()
    if not wrapped:
        sys.exit("TAURI_SIGNING_PRIVATE_KEY is empty")
    password = os.environ.get("TAURI_SIGNING_PRIVATE_KEY_PASSWORD", "")

    lines = base64.b64decode(wrapped).decode().splitlines()
    if len(lines) < 2:
        sys.exit("the secret key box has no key line")
    comment, encoded = lines[0], lines[1]
    key = bytearray(base64.b64decode(encoded))
    if len(key) != KEY_LEN:
        sys.exit(f"the secret key is {len(key)} bytes, expected {KEY_LEN}")

    kdf = bytes(key[2:4])
    if kdf == KDF_SCRYPT:
        decrypt(key, password)
    elif kdf != KDF_NONE:
        sys.exit(f"unsupported key derivation {kdf!r}")
    if not checksum_matches(key):
        sys.exit("wrong password for the update signing key")

    key[2:4] = KDF_NONE
    key[6:HEADER_LEN] = bytes(HEADER_LEN - 6)
    box = f"{comment}\n{base64.b64encode(key).decode()}\n"

    fd = os.open(sys.argv[1], os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w", newline="\n") as out:
        out.write(base64.b64encode(box.encode()).decode())


if __name__ == "__main__":
    main()
