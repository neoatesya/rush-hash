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

### 2. Konfigurasi `.env`
Pastikan file `.env` sudah ada di folder ini (`rush-hash/.env`) dan berisi:
```env
RPC_URL=https://ethereum-rpc.publicnode.com
PRIVATE_KEY=0xPRIVATE_KEY_WALLET_KAMU
PRIORITY_FEE_GWEI=2
```
> **⚠️ PENTING:** Jangan gunakan private key dompet utamamu. Gunakan dompet khusus mining dan pastikan ada saldo ETH untuk membayar biaya *gas*.

### 3. Build & Jalankan
Compile dan jalankan program dengan optimalisasi maksimal:
```bash
cargo run --release
```

---

## Log Eksekusi
Program akan otomatis mencetak status ke layar dan menyimpannya ke dalam file `miner.log`. Kamu bisa membiarkan program ini berjalan di *background* dan mengecek log kapan saja untuk melihat hasil *minting*.
