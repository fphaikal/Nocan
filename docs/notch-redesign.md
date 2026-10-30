# Tampilan notch Nocan

Arahan terbaru pengguna: tampilan notch modern Nocan dengan gaya Apple/macOS, seluruh aksen antarmuka mengikuti putih, abu-abu netral, dan biru. Maskot tetap memakai mata, corak dahi, dan tiga kumis tiap sisi berwarna putih. Pola donat tetap bulat di tengah dengan titik sampai ke tepi.

## Susunan umum

- Header: satu baris berisi ikon dan nama section, tanggal, jam, serta ikon daya putih. Header merek dan tagline sebelumnya dihapus untuk memberi ruang pada isi.
- Navigasi: bagian bawah berisi deretan bar pendek, satu bar untuk setiap section yang aktif. Bar section yang sedang dibuka lebih panjang dan terang. Bar dapat diklik; roda mouse dan panah kiri/kanan juga tetap berpindah section. Ikon bawah dan tombol panah bawah tidak dipakai. Nama section aktif tetap berada di header.
- Ukuran terbuka: badan 448 × 196 piksel. Mode lirik memakai lebar 560–700 dengan tinggi yang sama. Bar ringkas 188 × 32 menempatkan jam di tengah, baterai di kiri, dan ikon musik/aktivitas di kanan. Kanvas tetap 760 × 240.
- Palet: dasar hitam, gradasi sian–biru–ungu dari renderer maskot awal proyek, serta cahaya yang memudar ke atas. Semua latar memakai sumber palet yang sama di `src/graphics/ambient.rs`. Aksen Nocan kini putih, abu-abu netral, dan biru; tombol utama memakai biru lebih gelap agar teks putih terbaca. Logo penyedia AI mempertahankan warna mereknya. Maskot memakai corak dahi, mata, dan kumis putih; titik latar mengikuti biru muda.
- Siluet: radius sudut semula dipulihkan, 14 piksel saat tertutup dan 32 saat terbuka. Lengkungan sambungan ke tepi layar kembali 10 dan 22 piksel; interpolasi transisi memakai ukuran yang sama.
- Kontrol: play/pause, lirik, reset, pengingat, tipe berkas, status daya, dan logo penyedia AI tetap berbasis simbol. Tugas memakai tombol tambah/edit berupa ikon dan centang bulat; timer dimulai/dijeda dengan menekan tombol di dalam cincin.
- Tipografi: SF Pro Text menjadi default sesuai arahan pengguna untuk memakai font Apple. Renderer mendeteksi SF Pro Text, SF Pro, atau SF Pro Display yang terpasang; bila tidak tersedia, fallback ke Segoe UI Variable Text, Segoe UI Variable Display, lalu Segoe UI. Berkas font Apple tidak disertakan dalam aplikasi. Pengguna dapat memilih SF Pro (Apple) lewat Pengaturan > Tampilan; pilihan font lain yang tersimpan tetap dihormati. Angka utama memakai bobot regular/light, judul semibold, dan teks sekunder lebih redup untuk hierarki visual.
- Gerak: transisi section mempertahankan spring/crossfade; gerak maskot dan pola titik tetap halus. ENERGY 2/5, RHYTHM 2/5, MOTION 2/5.

## Susunan setiap section

| Section | Susunan dan fungsi |
| --- | --- |
| Musik | Padding samping 24 px; sampul 80 px dan gap 16 px menuju metadata. Judul, artis, progres, dan waktu sejajar dalam kolom kanan. Play 36 px menjadi aksi utama, diapit previous/next; lirik berada di bawah sampul. Mode lirik memakai sampul 56 px dan pemisah panel. |
| Fokus | Cincin progres dengan tombol play/pause di kiri; timer, mode aktif, dan status sesi di kanan; reset di kanan atas; tiga pilihan mode di bawah dengan ikon berbeda. |
| Air | Jumlah konsumsi dan target di kiri; grafik konsumsi per jam serta pengingat di kanan. Aksi +250 ml dan penggantian target tetap tersedia. |
| Berkas | Tiga thumbnail horizontal dengan nama dan ukuran, pagination, serta aksi kosongkan rak. Kondisi kosong memakai drop zone. Klik berkas menggunakan fungsi pembuka yang sudah ada. |
| Tugas | Ringkasan selesai, ikon tambah/edit, dan baris tugas dengan centang bulat serta pemisah tipis. Daftar kosong tidak diisi tugas fiktif. |
| OpenMaus | Tab Aktivitas, Jadwal, dan Penggunaan; riwayat agen/automation, jadwal berikutnya, serta rincian bot/model/provider. Lebar 560 dan tinggi 232 menyesuaikan secara dinamis ketika berpindah section. |
| AI Usage | Pemilih empat provider di atas; token hari ini dan status di kiri, model serta rincian token atau kuota di kanan. Garis pemisah menjaga hierarki tanpa kartu bertumpuk. |
| Saham | Delapan pemilih simbol, harga/perubahan di kiri dan sparkline di kanan. Data bawaan diberi label simulasi lokal. |

## Kode dan interaksi

`src/sections/deck.rs` merupakan renderer aktif untuk seluruh section. Setiap tombol menghasilkan `Region` dari persegi yang sama dengan yang digambar, lalu handler di `src/notch/window.rs` menerjemahkan `Action` ke layanan/state yang sudah ada. Fungsi gambar dan koordinat lama pada berkas section tidak lagi dipakai oleh renderer utama.

Form ticker pada Settings memakai bidang membulat setinggi 36 px, padding teks 12 px, font yang mengikuti preferensi antarmuka, serta ring biru saat fokus. Native EDIT hanya mengisi area teks bagian dalam; bingkai, pemilih IDX/Global, dan tombol Tambah digambar selaras oleh renderer Settings. Koordinat bersama berada di `src/settings/stock_input.rs`. Placeholder menyesuaikan pasar dan tema; Enter menambahkan ticker, sedangkan Tab berpindah ke kontrol berikutnya.

Footer menggunakan `src/notch/dots.rs::draw_icons` dan `hit_test`. Jumlah serta posisi bar diturunkan dari `total_sections`; lebar bar aktif 16 piksel dan bar lainnya 4,5 piksel dengan jarak 5 piksel. `set_active_sections` memperbarui jumlah bar ketika preferensi section berubah. Perubahan tata letak footer harus menjaga kesamaan posisi gambar dan target klik.

- Tab / Shift+Tab: pindah fokus antar kontrol section; outline putih hanya ditampilkan setelah navigasi keyboard.
- Enter / Space: menjalankan kontrol yang sedang fokus.
- Panah kiri/kanan atau roda mouse: pindah section.
- Escape: menutup notch.
- Klik mouse menjalankan tindakan tanpa meninggalkan penanda fokus. Penanda fokus keyboard berupa garis putih mengikuti kontrol yang sedang fokus.

Fungsi media, pembukaan berkas/editor, pelacakan pemakaian, penyimpanan tugas, dan pencatatan air tetap memakai layanan sebelumnya. Pratinjau diagnostik memeriksa mutasi state lokal tanpa mengirim perintah transport musik atau membuka aplikasi eksternal.

## Pratinjau

Jalankan executable dengan `--preview-branding <folder-output>` menggunakan profil `LOCALAPPDATA` terpisah. Mode ini merender delapan section, lirik, status daya, serta 100 frame maskot, kemudian memeriksa handler native untuk navigasi dan tindakan lokal.

Mode `--preview-branding <folder-output>` mengekspor frame setiap section dari renderer native. Data contoh diagnostik memakai profil pengujian terpisah.

Pratinjau revisi section Fokus berada di `assets/mascot/previews/nocan-focus-v13.png`; galeri seluruh section: `nocan-sections-v13.png`. Executable: `target/release/nocan-v13.exe`.

## Perapian jarak — 28 Oktober 2026

Header memakai inset 24 px dan pusat vertikal 20 px. Panel dibuka setinggi 196 px agar kontrol musik berakhir pada y=164 dan navigasi berada di y=178. Indikator section memakai gap 7 px; target klik mengikuti posisi baru. Pemeriksaan meliputi render native, area klik, serta mode lirik.

## Warna sampul media

Saat media diputar dan sampul tersedia, warna dominan sampul mengubah gradasi latar semua section. Transisi berlangsung halus; jika media dijeda, sampul tidak tersedia, atau Dynamic Ambience Glow dinonaktifkan, latar kembali ke palet semula. Area teks tetap berlatar gelap.

## Integrasi Settings dan ikon

Ikon aplikasi dan logo Settings menggunakan desain macan Nocan: telinga, corak dahi serta kumis putih di atas latar abu bergradasi. Aset `nocan.png`, `nocan.ico`, dan `nocan-64.bgra` tersedia di `assets/mascot/runtime`.

Perubahan Settings diterapkan ke notch melalui `WM_SETTINGS_CHANGED`: pilihan seksi dan provider, durasi fokus, target dan interval pengingat air, mode ciut, warna aksen, intensitas gradasi, font, serta lebar lirik. Indikator proporsi AI memakai token hari ini. Discord mengikuti pengaturan aplikasi selama konfigurasi Discord yang terpisah mengizinkannya. Startup Windows disimpan di Run milik pengguna saat sakelar diubah. Diagnostik `--preview-branding` menggunakan profil terpisah dan memeriksa penerapan pesan Settings tanpa mengubah pengaturan pengguna.

## Timer Fokus di luar panel

Saat timer Fokus berjalan, bilah ringkas mengganti jam dengan ikon timer dan hitungan menit/detik. Jika lirik sedang tampil pada bar ringkas, timer tidak dimasukkan ke dalam notch lirik; timer tampil pada capsule mandiri di sebelah kanan notch. Capsule menampilkan cincin progres, waktu tersisa, mode Fokus/Rehat, serta tombol jeda dan lewati yang bekerja langsung. Capsule memakai lebar 206 px, tinggi 58 px, jarak 10 px dari notch, dan latar gradasi aksen yang sama dengan notch. Saat notch dibuka, capsule disembunyikan agar tidak menutupi panel section. Pratinjau native: `target/focus-preview/focus-capsule.png` dan `target/focus-preview/compact-focus.png`.

## Desain Bento Grid Pengaturan — 28 Oktober 2026

Antarmuka Settings dirombak menggunakan struktur Bento Grid modular:
- Header: logo avatar macan Nocan, tipografi semibold, badge status sinkronisasi real-time (`Tersinkronisasi ke Notch`), serta navigasi capsule segmented bar dengan transisi slider halus.
- Seksi disusun dalam kartu-kartu bento mandiri dengan micro-badge ikon, toggle switch terpadu, kartu hero asimetris, dan angka metrik besar pada widget Pomodoro dan target hidrasi.
- Pratinjau render native Direct2D untuk 8 tab tersimpan di `target/settings-preview/settings-00-umum.png` sampai `settings-07-tentang.png`.
