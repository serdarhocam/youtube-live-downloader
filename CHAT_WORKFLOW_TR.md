# Canlı yayın sohbeti ve Resolve iş akışı

1. Canlı yayın kaydını indirin. yt-dlp yayını tanırsa video tamamlandıktan sonra sohbet tekrarı ayrıca indirilir. Sohbet hatası videonun tamamlandı durumunu değiştirmez. Eski kayıtlarda **Chat Kaydı → Sohbeti İndir** kullanın; videoyu tekrar indirmek gerekmez.
2. **Chat Kaydı** bağımsız bir pencere açar. Solda türlere göre mesaj ve katılımcı sayıları, en yoğun anlar ve ücretli mesaj zamanları bulunur. Sayımlar video zamanına bağlanabilen, yinelenmeyen mesajlar içindir. Zamanı bulunamayan ve okunamayan kayıt sayıları ayrıca gösterilir.
3. Yoğunluk, `[başlangıç, başlangıç + 15 saniye)` aralıklarıyla, 1 saniye adım kullanılarak hesaplanır. En fazla 50 zirve gösterilir; çakışan aralıklar aynı anı tekrar listelememek için elenir. Listeye tıklamak sohbeti o ana götürür. Mesaj zamanına tıklamak orijinal videoyu o saniyede açar.
4. Sağdaki mesaj listesi aranabilir ve türe göre filtrelenebilir. Uzun mesajın tamamı metnine tıklanınca altta açılır. **Ham JSON**, dosyayı 20 satırlık sayfalarla salt okunur gösterir.

## Resolve kesimlerini getirme

Resolve'da orijinal indirilen videodan kurgunuzu oluşturun. **File → Export → Timeline… → Final Cut Pro 7 XML** seçin. Videoyu render etmeniz gerekmez. Uygulamada **Resolve ve sohbet katmanı → Resolve XML İçe Aktar** kullanın.

- İlk sürüm tek etkin video kanalı, kaynak sırasını koruyan düz kesimler ve normal hız için tasarlanmıştır. Ses kanalları sohbet eşleştirmesinde kullanılmaz. Zaman çizelgesi boşlukları korunur.
- Birden çok video kanalı, geçişler, klip tekrarları/ters sıralama, iç içe timeline ve hız değişimleri reddedilir. Kurgu dosyasındaki kaynak adı mevcut videoyla eşleşmelidir.
- Zamanlamayı değiştirmeyen görsel efektler sohbet eşleştirmesini etkilemez. Kaynak ve timeline kare hızları ayrı kullanılır; NTSC 1000/1001 oranı dikkate alınır.
- XML, uygulamada “Resolve kesimlerini uygula” seçiliyken kullanılır. Seçimi kaldırmak orijinal sohbet zamanlamasına döner. XML'in kendisi ve ham sohbet değiştirilmez.
- Timeline başlangıç timecode'u 01:00:00:00 olsa da çıktı, timeline'ın ilk karesine yerleştirilir. Kurguyu değiştirince XML'i yeniden içe aktarın ve yeni sohbet çıktısı oluşturun.

## Kesilen aralığın sohbeti

Örneğin 02:00-02:30 kaldırıldıysa kaynakta 02:40'ta gelen mesaj kurguda 02:10'a taşınır. Çıkarılan aralıktaki son 5 mesaj, kesimden sonra “Önceki” etiketiyle en fazla 8 saniye görünür. Sayı ve süre ayarlanabilir; sıfır seçilerek kapatılabilir. Yeni mesajlar eskileri yukarı iter. Ham arşivde tüm mesajlar korunur; widget yalnız ayarlanan son mesajları gösterir.

## Çıktılar

- **Şeffaf MOV Oluştur:** QuickTime Animation (`qtrle`, ARGB) video; sohbet baloncukları ve alfa kanalı içerir. Resolve'da üst video kanalına, timeline'ın başlangıcına yerleştirin. Gerekirse klip alfa yorumunu **Straight** olarak kontrol edin. **15 sn MOV Örneği**, zaman çizelgesinin ilk 15 saniyesini üretir; başlangıçta mesaj yoksa örnek de boş olabilir.
- **SRT Kaydet:** Tüm mesajlar, yalnız Super Chat/Super Sticker veya yoğun an özetleri. SRT, altyazı zamanlaması sağlar; widget tasarımını taşımaz. Aynı anda gelen mesajlar aynı altyazı girdisinde birleştirilir.
- Genişlik, yükseklik, yazı boyutu, görünen mesaj sayısı, mesaj süresi ve sohbet zaman kaydırması ayarlanabilir. Orijinal zamanlama için FPS seçilir; XML kullanılırsa XML'in timeline FPS'i geçerlidir.
- Kullanıcı adları renklidir; ücretli mesajlar tutarıyla vurgulanır. Farklı para birimleri toplanmaz. YouTube emote ve sticker görselleri önizlemede ve MOV çıktısında gösterilir (hareketli görseller MOV için ilk kareye dönüştürülür). Görseller ilk dışa aktarımda `chat-emotes` klasörüne alınır, sonraki çıktılarda önbellekten kullanılır. SRT görsel desteklemediği için emote açıklamalarını metin olarak korur. Nick ve mesaj aynı satırdan başlar; metin widget genişliğinde sarılır. Mesaj kutusu arka planı yoktur; koyu kontur ve gölge okunabilirliği artırır. Dikey alan dolunca eski mesajlar üstten çıkar.
- Dışa aktarım iptal edilebilir. Uzun kayıtlarda MOV büyük olabilir. Her dışa aktarım yeni bir `chat-export-...` klasörü oluşturur; önceki çıktılar korunur. İptal edilen işlemin yarım dosyası aynı klasörde kalabilir.

## Dosyalar ve sınırlar

Videonun klasöründe `chat-VIDEO_ID.raw.jsonl` ham yt-dlp çıktısı, `chat-VIDEO_ID.processed.json` işlenmiş mesajlar ve `chat-VIDEO_ID.status.json` işlem durumu bulunur. Ham dosya mevcutsa yeniden indirilip üzerine yazılmaz. İndirme sırasında oluşan `.chat-download-...` klasörleri günlükleri ve varsa kısmi dosyaları saklar. XML içe aktarımı zamanlama haritasını `chat-VIDEO_ID.resolve.json` olarak yazar; seçilen XML'in ayrı bir kopyasını da saklar.

YouTube sohbet tekrarı sunmuyorsa uygulama bunu bildirir. Silinmiş/erişilemeyen mesajların geri getirilmesi garanti edilmez. Devam eden yayınların sohbetini canlı izleme bu sürümün kapsamı dışındadır.

Doğrulama: gerçek bir YouTube sohbet arşivi, ham dosya bütünlüğü, XML/NTSC ve kesim testleri, arayüz araması ve alfa kanallı örnek MOV kontrol edildi. Kullanıcının Resolve kurulumunda gerçek timeline ve MOV içe aktarma kontrolü ayrıca yapılmalıdır.

## 1.9.0: süreler ve geçmiş mesajlar

- Mesaj ve geçmiş mesaj süreleri kesim sınırlarında sıfırlanmaz; yeni timeline üzerinde işler ve video sonunda biter. Mesaj sayısı sınırı veya widget yüksekliği dolarsa en eski mesajlar daha erken görünümden çıkabilir.
- Geçmiş mesajlarda etiket yoktur. Yazı boyutu ve opaklık (0: görünmez, 1: tamamen opak) ayrı ayarlanır; önizleme ve MOV aynı ayarları kullanır.
- MOV ve SRT, “Resolve kesimlerini uygula” açıkken XML sonrası zamanları kullanır. Seçim hatırlanır; dışa aktarım alanında aktif zaman çizelgesi yazılıdır. SRT görsel stilleri taşımaz ve çakışmayan altyazı için bir sonraki mesajda önceki altyazıyı bitirebilir.
- Çıktı hazır olunca “Çıktı Klasörünü Aç” doğrudan oluşturulan dosyanın klasörünü açar.
