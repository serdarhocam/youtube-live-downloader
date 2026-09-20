# YouTube Live Downloader - Windows için Portable YouTube Video İndirici

[![Son Sürüm](https://img.shields.io/github/v/release/serdarhocam/youtube-live-downloader?label=s%C3%BCr%C3%BCm)](https://github.com/serdarhocam/youtube-live-downloader/releases/latest)
[![Lisans: MIT](https://img.shields.io/badge/Lisans-MIT-green.svg)](LICENSE)

**[Portable Windows uygulamasını indir](https://github.com/serdarhocam/youtube-live-downloader/releases/latest)** · [SHA-256 doğrulama dosyası](https://github.com/serdarhocam/youtube-live-downloader/releases/latest)

Güncel uygulama sürümü: **1.9.1 / p0009**

[English README](README.md)

Windows 10/11 için ücretsiz ve portable YouTube video indirici. Herkese açık YouTube videolarını, oynatma listelerini, kanal yayınlarını ve arşivlenmiş canlı yayınları ek kurulum olmadan indirin. Bu portable YouTube indirici Python, Node.js, yt-dlp veya FFmpeg kurulumu istemez; gerekli araçları kendi içinde taşır ve ayrı video/ses akışlarını gereksiz yeniden kodlama yapmadan birleştirir.

Yalnızca sahibi olduğunuz veya indirme izniniz bulunan içerikler için kullanın. Uygulama DRM, kimlik doğrulama veya CAPTCHA mekanizmalarını aşmaz.

## Özellikler

- yt-dlp JSON aracılığıyla video, Shorts, oynatma listesi ve `/streams` adresi meta verileri
- Küçük resim, başlık, tarih, süre, kanal, açıklama, çözünürlük ve FPS gösterimi
- Tekli/çoklu seçim ve varsayılan iki eşzamanlı indirme kuyruğu
- En iyi kalite veya üst çözünürlük sınırı seçimi
- Gerçek byte, hız, ETA, yüzde ve birleştirme aşaması ilerlemesi
- İş bazında iptal ve yt-dlp `.part` devam desteği
- Uygulama yeniden başlatmalarında korunan kuyrukla gerçek Duraklat/Devam Et/Tümünü Devam Ettir kontrolleri
- Tamamlanmış, başarısız, duraklatılmış, kuyrukta veya aktif işler için listeden kaldırma
- Yerel yt-dlp parça tekrar denemeleri ve uygulama seviyesinde geçici hata geri çekilme politikası
- Yerel Windows klasör seçici ve kalıcı ayarlar
- Yapılandırılmış hedefi Windows Gezgini'nde tek tıklamayla açma
- İsteğe bağlı JPG küçük resim, güvenli Windows dosya adları ve yanlışlıkla üzerine yazmayı önleme
- Kullanıcı dostu hatalar ve açılabilir teknik ayrıntılarla anonim kullanım
- Tauri uygulama log klasöründe günlük yerel loglar
- Kullanıcı bazında saklanan anlık Türkçe/English dil seçimi
- Yalnız gerçekten uzun sonuç listelerinde kaydırma gerektiren kompakt responsive başlangıç yerleşimi
- Başlıkta ve Hakkında penceresinde bağlantılı SERDARHOCAM geliştirici bilgisi

## Mimari

Arayüz Vite ile oluşturulan TypeScript, HTML ve CSS kullanır. Rust backend ile yalnızca tipli Tauri komutları ve olayları üzerinden iletişim kurar. URL doğrulama, süreç çalıştırma, ayar saklama, kuyruk, iptal, ilerleme ayrıştırma ve loglama Rust tarafından yapılır. Ham URL'ler ayrı süreç argümanları olarak iletilir; kullanıcı girdisinden hiçbir zaman shell komutu oluşturulmaz.

`src-tauri/src/services/` altındaki önemli backend modülleri:

- `binaries.rs`: Paketlenmiş çalıştırılabilir dosyaları ve geliştirme kopyalarını çözümler.
- `ytdlp.rs`: Meta veri/sürüm sorgularını ve JSON dönüşümünü yönetir.
- `downloader.rs`: Format seçimi, kuyruk, süreçler, ilerleme ve iptali yönetir.
- `settings.rs`: Ayarları normal kullanıcı uygulama yapılandırma klasöründe JSON olarak saklar.

## Son kullanıcı bağımlılıkları

Yoktur. Kullanıcının Node.js, npm, Python, Rust, Tauri, yt-dlp veya FFmpeg kurması gerekmez. Ana portable EXE yt-dlp ve FFmpeg'i kendi içinde taşır ve doğrulanmış kopyalarını otomatik çıkarır. Windows 10/11 normal olarak gerekli Microsoft Edge WebView2 çalışma zamanını içerir.

## Geliştirici bağımlılıkları

- Windows 10/11
- Node.js 20 veya daha yeni
- Rust stable MSVC toolchain
- Desktop development with C++ ve Windows SDK içeren Visual Studio 2022 Build Tools
- WebView2 runtime
- Depoyu klonlamak veya GitHub'a göndermek için Git LFS (FFmpeg dosyaları GitHub'ın normal tek-dosya sınırını aşar)

JavaScript paketlerini kurun:

```powershell
npm install
```

İlk GitHub checkout veya ilk push öncesinde:

```powershell
git lfs install
git lfs pull
```

Kapsamı dar `.gitattributes` kuralları yalnızca gömülü EXE dosyalarını LFS'te saklar. Derlemeden önce `src-tauri\binaries\*.exe` dosyalarının küçük LFS pointer metinleri değil, gerçek `MZ` çalıştırılabilir dosyaları olduğunu doğrulayın.

## Gömülü çalıştırılabilir dosyalar

Depo kopyaları `src-tauri/binaries/` altında bulunur:

- `yt-dlp.exe`: Resmi [yt-dlp sürümleri](https://github.com/yt-dlp/yt-dlp/releases)
- `ffmpeg.exe` ve `ffprobe.exe`: FFmpeg kaynağından derlenen [gyan.dev Windows essentials build](https://www.gyan.dev/ffmpeg/builds/)
- `VERSIONS.txt`: Sabitlenmiş sürümler ve kaynak adresleri

Rust binary yöneticisi yt-dlp ve FFmpeg'i `include_bytes!` ile doğrudan uygulamaya bağlar. Başlangıçta gömülü yükün hash'ini hesaplar ve araçları `%LOCALAPPDATA%\com.ytld.desktop\embedded-binaries\v1\<hash>` altına çıkarır. Var olan dosyalar yalnız boyut ve SHA-256 doğrulamasından sonra yeniden kullanılır. Çıkarma işlemi süreçler arası kilit, geçici dosya, flush, doğrulama ve atomik yeniden adlandırma kullanır. yt-dlp'ye her zaman `--ffmpeg-location` ile çıkarılmış FFmpeg klasörü verilir; hiçbir araç `PATH` içinde aranmaz.

Mevcut NSIS yükleyicinin isteğe bağlı dağıtım olarak çalışmaya devam etmesi için aynı kaynak binary'ler `tauri.conf.json` içinde Tauri resource olarak da listelenir. Portable EXE bu dış resource kopyalarına bağımlı değildir.

yt-dlp'yi güncellemek için `src-tauri/binaries/yt-dlp.exe` dosyasını doğrulanmış resmi sürümle değiştirin, `VERSIONS.txt` dosyasını güncelleyin, `--version` komutuyla doğrulayın ve yeniden derleyin. Kaynak kod değişikliği gerekmez.

## Geliştirme

```powershell
npm install
npm run check
npm run tauri dev
```

Backend kontrolleri:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## Derleme cache'i ve disk kullanımı

Rust; derlenmiş bağımlılıkları, debug çıktılarını, release ara dosyalarını ve incremental derleme verilerini `src-tauri/target` altında saklar. Portable uygulama büyük yt-dlp ve FFmpeg yüklerini `include_bytes!` ile gömdüğü için tekrarlanan debug/release derlemeleri bu klasörü olağan dışı ölçüde büyütebilir. Bu klasör kaynak kod veya kullanıcı indirme verisi değil, yeniden üretilebilir derleme cache'idir.

Alanı güvenle geri kazanmak için:

```powershell
cargo clean --manifest-path src-tauri/Cargo.toml
```

Bu komut yalnız `src-tauri/target` altındaki Rust derleme çıktılarını kaldırır. Kaynak dosyaları, `src-tauri/binaries`, önceden kopyalanmış `portable/`, kullanıcı indirmeleri, ayarlar veya kalıcı indirme kuyruğu silinmez. Tüm bağımlılıkların yeniden derlenmesi gerektiği için sonraki Rust/Tauri derlemesi daha uzun sürer. `src-tauri/target/` `.gitignore` ile Git dışında tutulur ve hiçbir zaman commit edilmemelidir.

## Sürüm ve paket numaralı dağıtım

- `npm run portable`: Yalnız portable EXE üretir.
- `npm run installer`: Yalnız NSIS kurulum paketini üretir.
- `npm run release`: Portable ve kurulum paketini aynı sürüm/paket numarasıyla üretir.

Örnek çıktılar:

- `portable/YouTubeLiveDownloader-v1.6.0-p0001-portable.exe`
- `installer/YouTubeLiveDownloader-v1.6.0-p0001-installer.exe`

Her EXE yanında aynı adı taşıyan `.exe.sha256` doğrulama dosyası oluşturulur. Paket numarası `scripts/package-sequence.json` içinde tutulur, her paketleme çalışmasında otomatik artar ve sürüm değiştiğinde sıfırlanmaz. Başarısız derlemelerin numarası tekrar kullanılmaz. Bu sayaç dosyasını release değişiklikleriyle birlikte commit edin. Eşzamanlı paketlemeler kilitle engellenir; eski paketlerin üzerine yazılmaz.

Dağıtım için bu komutları kullanın. Doğrudan Tauri komutları `src-tauri/target` altında ham ara çıktılar üretir; dağıtılacak sürüm/paket numaralı dosyalar `portable` ve `installer` klasörlerindedir. Eski, numarasız EXE önceki sürüm olarak kalır ve güncellenmez.

Geliştirme: `npm run tauri dev`. Yayın öncesi kontrol: `npm run check` ve `cargo test --manifest-path src-tauri/Cargo.toml --lib`.

## Sürümleme ve yayın akışı

Semantik sürümleme kullanılır:

- `PATCH`: Beklenen davranışı değiştirmeyen hata düzeltmesi.
- `MINOR`: Geriye uyumlu özellik veya önemli güvenilirlik geliştirmesi.
- `MAJOR`: Ayar, kuyruk veya kullanıcı davranışında kırıcı değişiklik.

Tüm senkron proje sürümlerini güncelleyin:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/set-version.ps1 -Version 1.4.0
npm install --package-lock-only
```

Ardından `CHANGELOG.md` dosyasını güncelleyin, tüm release kontrollerini çalıştırın, commit oluşturun, `v1.4.0` gibi bir Git etiketi ekleyin ve portable EXE, SHA-256 dosyası ile istenirse NSIS yükleyiciyi GitHub Release'e ekleyin. Uygulama, npm paketi, Cargo paketi ve Tauri bundle sürümleri aynı olmalıdır.

Gömülü araç sürümleri ayrıca `src-tauri/binaries/VERSIONS.txt` içinde takip edilir. yt-dlp veya FFmpeg güncellemesi uygulama kodu değişikliği gerektirmez; kullanıcıların yeni yükü ayırt edebilmesi için yeni bir patch release önerilir.

## Depo politikası

Temiz checkout'un çevrimdışı son kullanıcı derlemesi oluşturabilmesi için şunlar bilerek commit edilir:

- Rust, TypeScript, CSS ve derleme scriptleri
- `package-lock.json` ve `src-tauri/Cargo.lock`
- `src-tauri/binaries/yt-dlp.exe`
- `src-tauri/binaries/ffmpeg.exe` ve `ffprobe.exe`
- `src-tauri/binaries/VERSIONS.txt`

Üretilmiş `node_modules`, `dist`, Rust target klasörleri, portable çıktılar, indirilen arşivler, doğrulama medyası, `.part` dosyaları, loglar ve makineye özgü çalışma durumu `.gitignore` ile hariç tutulur. Cookie, kimlik doğrulama verisi, indirilen videolar, kuyruk durumu veya kullanıcı ayarlarını commit etmeyin.

## Projeyi genişletme

Mevcut servis sınırları mimariyi değiştirmeden yeni özellik eklemeye uygundur. Olası geliştirmeler: açıkça isteğe bağlı tarayıcı cookie kimlik doğrulaması, altyazı seçimi, yalnız ses indirme, oynatma listesi filtreleri, indirme geçmişi temizliği, imzalı güncellemeler, ek diller ve otomatik GitHub Actions release derlemeleri. Kimlik doğrulama aşma, DRM çözme ve CAPTCHA aşma kapsam dışıdır.

Yeni özelliklerde süreç çalıştırmayı Rust servisleri içinde tutun, argümanları shell kullanmadan geçirin, embedded-binary yöneticisini koruyun, kalıcı kuyruğa eklenecek alanları geriye uyumlu Serde varsayılanlarıyla ekleyin ve hem portable EXE'yi hem isteğe bağlı yükleyiciyi doğrulayın.

## Ayarlar ve loglar

Varsayılan hedef `Videos\YouTube Live Downloader` klasörüdür; mümkün değilse Downloads kullanılır. İndirme klasörü, varsayılan kalite, eşzamanlı indirme sayısı, küçük resim tercihi ve arayüz dili Tauri'nin kullanıcı bazlı uygulama yapılandırma konumunda saklanır. Loglar kullanıcı bazlı Tauri log konumundadır ve cookie veya kimlik doğrulama verisi içermez.

`com.ytld.desktop` için tipik Windows konumları:

- Ayarlar ve kalıcı kuyruk: `%APPDATA%\com.ytld.desktop`
- Çıkarılmış gömülü araçlar: `%LOCALAPPDATA%\com.ytld.desktop\embedded-binaries\v1\<payload-hash>`
- Loglar: Mevcut kullanıcının Tauri uygulama log klasörü

Bu konumlar uygulamaya ait çalışma depolamasıdır. Otomatik oluşturulur ve geliştirme deposundan bağımlılık değildir.

## Dil ve yerleşim

Başlıktaki dil seçici arayüzü anında **Türkçe** ve **English** arasında değiştirir. Seçim `settings.json` içine yazılır ve sonraki başlangıçta geri yüklenir. 1.2 öncesi mevcut ayar/kuyruk dosyaları uyumludur ve kullanıcı seçim yapana kadar English varsayılır.

Varsayılan pencere kompakt responsive yerleşimle 1280×860 boyutundadır. Kısa ekranlarda kart görselleri, boşluklar ve isteğe bağlı açıklama önizlemeleri otomatik küçülür; böylece başlangıç görünümü ve küçük geri yüklenmiş kuyruk gereksiz dikey scrollbar olmadan sığar. Kullanılabilir ekrandan gerçekten uzun oynatma listeleri ve kuyruklar kaydırılabilir kalır.

## Konsol penceresi politikası

Uygulama Windows GUI subsystem ile derlenir. Meta veri yükleme, sürüm sorgulama, indirme ve süreç ağacı iptal yardımcısı dahil tüm yt-dlp çağrıları `CREATE_NO_WINDOW` kullanır. Normal Yükle, Hakkında, İndir, Duraklat, İptal ve Kaldır işlemleri PowerShell veya Komut İstemi penceresi açmaz.

## Canlı ilerleme

yt-dlp açıkça `--progress`, `--newline`, `--progress-delta 0.25` ve JSON progress şablonuyla çalıştırılır. Açık `--progress` gereklidir; çünkü `after_move` print hook'u aksi durumda yt-dlp'nin quiet davranışını etkinleştirerek ara çıktıları susturur. Rust hem stdout hem stderr ilerlemesini okur, kesin indirilen/toplam byte alanlarından yüzdeyi hesaplar ve yapısal Tauri olayları gönderir. Arayüz tamamlanmayı beklemeden bar, yüzde, byte, hız ve ETA'yı günceller. Bazı YouTube aşamaları toplam boyut bildirmez; bu aşamalarda byte ve durum doğru gösterilir, yt-dlp toplamı verene kadar yüzde gösterilmez.

## Kuyruk kayıtlarını kaldırma

Her kalıcı indirme kartında **Kaldır** işlemi bulunur. Aktif bir iş kaldırıldığında önce yalnız o işe ait yt-dlp süreci durdurulur. Ardından iş `download-queue.json` ve görünür listeden çıkarılır. Tamamlanmış medya, `.part` dosyaları, fragment durumu, küçük resimler ve diskteki diğer dosyalar bilerek korunur. Böylece büyük kullanıcı dosyaları sessizce silinmez. Bir kaydı kaldırmak indirilen veriyi silmekten farklıdır; dosya temizliği uygulama dışında kullanıcının açık işlemi olarak kalır.

## Kesinti kurtarma

İndirme durumu Tauri kullanıcı uygulama yapılandırma klasöründeki `download-queue.json` dosyasına yazılır; normalde `%APPDATA%\com.ytld.desktop`. İlerleme sırasında yazımlar en fazla iki saniyede bir yapılır ve flush edilmiş geçici dosya ile yedek kullanılır. URL, öğe meta verisi, kalite, hedef, çıktı şablonu, ilerleme, zamanlar ve tekrar deneme durumu saklanır; süreç kimlikleri saklanmaz.

Uygulama yeniden başladığında aktif halde bulunan işler **Kesildi** durumuna çevrilir ve açık Devam Et/Tümünü Devam Ettir isteği bekler. Duraklat yalnız ilgili yt-dlp sürecini sonlandırır ve `.part`/fragment durumunu korur. İptal de kısmi veriyi korur ve planlamayı durdurur; daha sonra elle devam edilebilir. yt-dlp yeni ilerleme verene kadar geçmiş yüzde yaklaşık olarak etiketlenir.

Her indirme açıkça şunları kullanır:

```text
--continue --part --no-overwrites
--retries 10 --fragment-retries 20
--extractor-retries 5 --file-access-retries 5
--retry-sleep http:exp=1:20
--retry-sleep fragment:exp=1:20
--retry-sleep extractor:exp=1:10
--concurrent-fragments 4
```

yt-dlp yerel tekrar denemelerini tüketirse uygulama geçici hataları 60 saniye üst sınırına sahip üstel gecikmeyle en fazla on kez tekrar dener. Kalıcı erişilebilirlik, kimlik doğrulama ve format hataları tekrar döngüsüne girmez. Tükenmiş geçici hatalar ve disk dolu hataları devam ettirilebilir **Kesildi** işi olur. Deterministik çıktı şablonu, kesilmiş birleştirme sonrasında tamamlanmış bileşen akışlarının yeniden kullanılmasını sağlar; kaynak akışlar yalnız başarılı merge sonrasında yt-dlp tarafından kaldırılır.

## Sınırlamalar

- Yalnız herkese açık ve URL ile erişilebilen liste dışı içerik; v1'de login/cookie yoktur.
- YouTube sık değiştiği için gömülü yt-dlp güncel tutulmalıdır.
- Meta veri kullanılabilirliği öğeye göre değişir. Eksik açıklama, tarih, format veya boyutlar güvenli biçimde gösterilir.
- Oynatma listesi ve kanal adresleri; YouTube sunduğunda küçük resim, yükleme tarihi, süre ve format bilgisinin kullanılabilmesi için ayrıntılı öğe meta verisi ister. Bu nedenle çok büyük listelerin yüklenmesi daha uzun sürebilir ancak işlem UI thread'ini bloke etmez. Silinmiş/gizli öğeler ve YouTube'un tarih sunmadığı içerikler güvenli fallback ile gösterilir.

## Lisans

YouTube Live Downloader'ın kendi kaynak kodu [MIT Lisansı](LICENSE) ile sunulur. Birlikte dağıtılan yt-dlp ve FFmpeg çalıştırılabilir dosyaları kendi upstream lisanslarına tabidir; ayrıntılar için [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) dosyasına bakın.

## 1.6.0: Video klasörleri ve oynatıcı

Yeni indirmeler `İndirme klasörü/Video başlığı [video kimliği]/` altında video ve JPG kapağı birlikte tutar. Eski kayıtlar ve yarım indirmeler mevcut yollarını korur.

Her indirme kartındaki **Oynat** düğmesi bağımsız bir masaüstü penceresi açar. Başlat, duraklat, durdur, 10 saniye geri, 30 saniye ileri, 0,25-10× hız, ses ve tam ekran denetimleri bulunur. Oynatma, Windows WebView2 video/ses codec desteğine bağlıdır; desteklenmeyen dosyalarda oynatıcı hata gösterir.

**Klasörde Göster**, kaydın kendi klasörünü Explorer ile açar. Açılışta, ana pencereye dönüşte ve indirme tamamlandığında dosyalar kontrol edilir. Tamamlanan kayıtlarda video, beklenen JPG kapak veya klasör bulunamazsa kart üzerinde belirtilir. Eksik video oynatılamaz; eksik klasör yeniden oluşturulmaz.

## 1.7.0: Sohbet arşivi ve Resolve

Canlı yayın sohbetlerini indirme, 15 saniyelik yoğunluk analizi, Resolve XML kesimlerini içe aktarma ve şeffaf sohbet katmanı üretme için [kullanım rehberine](CHAT_WORKFLOW_TR.md) bakın.

## 1.9.1 / p0009

- [Portable EXE](https://github.com/serdarhocam/youtube-live-downloader/releases/download/v1.9.1/YouTubeLiveDownloader-v1.9.1-p0009-portable.exe)
- [Installer EXE](https://github.com/serdarhocam/youtube-live-downloader/releases/download/v1.9.1/YouTubeLiveDownloader-v1.9.1-p0009-installer.exe)
- [SHA-256](https://github.com/serdarhocam/youtube-live-downloader/releases/download/v1.9.1/SHA256SUMS.txt)
- [Chat ve Resolve rehberi](CHAT_WORKFLOW_TR.md)

YouTube Shorts indirmeleri artık yt-dlp'nin JavaScript challenge işlemleri için uygulamaya gömülü Deno çalışma zamanını kullanır; ayrıca kurulum gerekmez. Sohbet ve Resolve özellikleri değişmeden korunur.
