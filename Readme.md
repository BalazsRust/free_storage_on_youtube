# BinaryCoder

Fájlokat alakít QR-kódokból álló videóvá, és a videóból bitre pontosan vissza. Opcionálisan jelszóval titkosít. Rust nyelven írtam az OTIO 36. versenyére.

A projekt még fejlesztés alatt áll. A prototípus működik, a paraméterek hatásának szisztematikus mérése folyamatban van, eredményeket még nem közlök.

## Hogyan működik

Kódolás:

1. a fájl(ok) egy tar.gz csomagba kerülnek
2. opcionális AES-256-GCM titkosítás, a kulcsot Argon2 származtatja a jelszóból (véletlen só és nonce)
3. Base91 szöveggé alakítás
4. sorszámozott darabokra vágás
5. minden darabból színes, nagy kapacitású QR-kód (qrism, három kód az R, G, B csatornában)
6. több QR-kód egy videókockán
7. H.264 videó (ffmpeg, libx264)

Dekódolásnál a program képkockákra bontja a videót, beolvassa a kódokat, sorszám szerint összefűzi őket, visszafejt és kicsomagol. A képkockák sorrendje és az ismétlődés nem számít. Ha egy darab nem olvasható, a program jelzi, melyik hiányzik.

## Futtatás

Kell hozzá Rust és ffmpeg.

```
cargo run --release
```

Ez elindítja a HTTP API-t a 3000-es porton. Konténerből:

```
podman build -t binarycoder .
podman run -p 3000:3000 binarycoder
```

A webes felület egyetlen fájl (`binarycoder.html`), a `localhost:3000` címen lévő API-t hívja. Konténerben: `Containerfile.web`.

## API

Mindkét végpont multipart kérést vár. A `config` mező JSON, a többi mező a feltöltött fájl.

- `POST /encode` fájlok feltöltése, válasz: mp4 videó
- `POST /decode` videó feltöltése, válasz: tar.gz a visszaállított fájlokkal

Példa:

```
curl -F 'config={"encrypt":true,"password":"jelszo","error_correction":1,"crf":28,"fps":30,"byte_len":3000}' \
     -F 'file=@titok.pdf' http://localhost:3000/encode -o output.mp4

curl -F 'config={"encrypt":true,"password":"jelszo","error_correction":1,"crf":28,"fps":30,"byte_len":3000}' \
     -F 'video=@output.mp4' http://localhost:3000/decode -o vissza.tar.gz
```

Fontos: ha megadsz `config` mezőt, add meg benne az összes értéket. A kihagyott mezők nem az alapértékeket kapják (például a `crf` 0, az `error_correction` 0 lesz), és dekódoláskor a `password` és a felbontás is kell.

## Paraméterek

| Név | Alapérték | Jelentés |
|---|---|---|
| `error_correction` | 1 | QR hibajavítás: 0 = L, 1 = M, 2 = Q, 3 = H |
| `byte_len` | 5000 | egy QR-kód adatmennyisége bájtban (legfeljebb 6496) |
| `crf` | 28 | H.264 minőség, kisebb érték jobb minőség és nagyobb fájl |
| `fps` | 240 | képkockasebesség |
| `frames_per_image` | 1 | ahány képkockán át látszik egy kép |
| `width`, `height` | 1920, 1080 | videó felbontása |
| `scale_to` | 1080 | egy QR-kód oldala pixelben |
| `encrypt` | false | titkosítás be/ki |
| `password` | | jelszó a titkosításhoz |

## Ismert hiányosságok

- A fájlnév titkosítatlanul kerül az első darabba.
- Hibás jelszónál a program hibaüzenet helyett leáll.
- A titkosítatlan módnak nincs saját sértetlenség-ellenőrzése.
- Parancssori mód jelenleg nincs bekötve, a `main` az API-t indítja.
- Nem teszteltem, hogyan viselkedik a videó, ha egy videómegosztó platform újratömöríti.

Az eszköz oktatási és kutatási célú. Videómegosztó platformon csak a platform felhasználási feltételeinek betartásával használd.

## Felhasznált könyvtárak

qrism, ffmpeg, aes-gcm, argon2, base91, axum, image, tar, flate2.
