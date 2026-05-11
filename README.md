# Rush Hash Miner (GPU OpenCL)

Miner kecepatan tinggi untuk [Hash256](https://hash256.org/mine) yang ditulis ulang dalam bahasa Rust dengan dukungan GPU (OpenCL).

## Fitur Utama
- **Kecepatan Tinggi**: Memanfaatkan GPU via OpenCL untuk menghitung jutaan *hash* per detik.
- **Auto-Boost Gas**: Otomatis menambahkan *Priority Fee* saat *nonce* ditemukan agar transaksi menang dari miner lain.
- **Challenge Tracking**: Berhenti dan memulai ulang pencarian otomatis jika *challenge* blok berubah.
- **Logging**: Menyimpan riwayat eksekusi dan hasil *mining* ke dalam file `miner.log`.

---

## Panduan Instalasi (Ubuntu/WSL)

### 1. Install Dependencies
Untuk menjalankan OpenCL di Ubuntu/WSL, kamu perlu menginstal library OpenCL:
```bash
sudo apt update
sudo apt install -y build-essential ocl-icd-opencl-dev clinfo
```

*(Opsional)* Jalankan `clinfo` untuk memastikan GPU kamu terdeteksi oleh sistem.

### 2. Install Rust (Cargo)
Karena program ini ditulis dalam bahasa Rust, kamu harus menginstal *compiler* Rust (yang di dalamnya terdapat `cargo`) jika lingkunganmu (misal: Google Colab/Ubuntu kosong) belum memilikinya:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

### 3. Konfigurasi `.env`
Program membutuhkan file konfigurasi bernama `.env`. Karena file ini berawalan titik, ia otomatis menjadi file tersembunyi (*hidden file*) di Linux (gunakan `ls -a` untuk melihatnya).

Pertama, gandakan file contoh (template) yang sudah disediakan:
```bash
cp .env.example .env
```

Selanjutnya, edit file `.env` tersebut. Jika kamu belum menginstall aplikasi teks editor `nano`, install terlebih dahulu dengan perintah:
```bash
sudo apt-get update && sudo apt-get install nano -y
```

Lalu buka filenya untuk diedit:
```bash
nano .env
```

Isi dengan data milikmu (bisa gunakan multiple RPC dipisah dengan koma untuk performa optimal):
```env
RPC_URL=https://ethereum-rpc.publicnode.com,https://rpc.mevblocker.io/fast,https://rpc.flashbots.net/fast
PRIVATE_KEY=0xPRIVATE_KEY_WALLET_KAMU
PRIORITY_FEE_GWEI=2
```
> **⚠️ PENTING:** Jangan gunakan private key dompet utamamu. Gunakan dompet khusus mining dan pastikan ada saldo ETH untuk membayar biaya *gas*.

### 4. Build & Jalankan
Compile dan jalankan program dengan optimalisasi maksimal:
```bash
cargo run --release
```

---

## Log Eksekusi
Program akan otomatis mencetak status ke layar dan menyimpannya ke dalam file `miner.log`. Kamu bisa membiarkan program ini berjalan di *background* dan mengecek log kapan saja untuk melihat hasil *minting*.

---

## ⚡ Khusus Pengguna Google Colab

Jika kamu mengeksekusi program ini di Google Colab, perhatikan 3 hal berikut:

1. **Edit `.env` Tanpa Terminal (Lebih Mudah)**
   Kamu tidak perlu repot-repot menggunakan `nano`. Setelah menggandakan file dengan `!cp .env.example .env`, cukup klik ikon **Folder (Files)** di panel paling kiri layar Colab, klik ganda pada file `.env`, lalu edit isinya langsung di dalam *browser*-mu dan tekan `Ctrl+S` untuk menyimpan.
   
2. **Path Cargo (`command not found`)**
   Setelah menginstal Rust di langkah ke-2, Colab biasanya tidak otomatis mendeteksi perintah `cargo`. Jika saat menjalankan `!cargo run --release` kamu mendapat error *command not found*, panggil `cargo` menggunakan *path* lengkapnya:
   ```bash
   !~/.cargo/bin/cargo run --release
   ```

3. **Gunakan Awalan `!`**
   Jangan lupa, setiap perintah terminal yang dijalankan di *cell* Colab harus diawali dengan tanda seru (`!`), contohnya `!sudo apt update` atau `!cp .env.example .env`.
