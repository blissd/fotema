## Terms

# See https://projectfluent.org/fluent/guide/terms.html

-app-name = Fotema

## Main Navigation Pages

# Title for library page, which contains the "all", "months", and "years" pages.
library-page = Kitaplık

# Title for years album.
years-album = Yıl

# Title for months album.
months-album = Ay

# Title for all photos/videos album.
all-album = Gün

# Title for video album.
videos-album = Videolar

# Title for album of selfies.
selfies-album = Özçekimler

# Title for album of iOS live photos and Android motion photos.
animated-album = Hareketli

# Title for album showing all folders.
folders-album = Klasörler

# Title for album showing contents of one folder.
folder-album = Klasör

# Title for places page which shows photos overlayed onto a map.
places-page = Yerler

## Thumbnail decorations

# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label = { $month ->
   [1] Ocak {$year}
   [2] Şubat {$year}
   [3] Mart {$year}
   [4] Nisan {$year}
   [5] Mayıs {$year}
   [6] Haziran {$year}
   [7] Temmuz {$year}
   [8] Ağustos {$year}
   [9] Eylül {$year}
   [10] Ekim {$year}
   [11] Kasım {$year}
   [12] Aralık {$year}
  *[other] {$year}
}

## About Dialog

# Section header for open source projects acknowledgements.
about-opensource = Açık Kaynaklı Projeler

# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits =
  queeup <queeup@zoho.com>

## Photo/Video Viewer

# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = Özellikleri göster

# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
  .tooltip = Sonraki

# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
  .tooltip = Önceki

# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
  .tooltip = Oynat/Duraklat

# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
  .tooltip = 10 Saniye İleri Atla

# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
  .tooltip = 10 Saniye Geriye Atla

# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
  .tooltip = Sesi Kapat/Aç

# Convert all incompatible videos description.
viewer-convert-all-description = Bu video oynatılmadan önce dönüştürülmelidir. Bunun yalnızca bir kez yapılması gerekir, ancak bir videoyu dönüştürmek biraz zaman alır.

# Button to convert all incompatible videos.
viewer-convert-all-button = Tüm uyumsuz videoları dönüştür

# Viewer failed to load an image or video.
viewer-error-failed-to-load = Yüklenemedi

# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file = Dosya eksik olduğu için görüntülenemiyor:
  {$file_name}

# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = Dosya yolu veritabanında mevcut değil

## Photo/Video Information Sidebar

# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = Klasör
  .tooltip = Dosyayı İçeren Klasörü Aç

# File name of photo or video
infobar-file-name = Dosya Adı

# File creation timestamp from file system metadata.
infobar-file-created = Dosyanın Oluşturulma Tarihi

# File modification timestamp from file system metadata.
infobar-file-modified = Dosyanın Değiştirilme Tarihi

# File size file system metadata.
infobar-file-size = Dosya Boyutu

# File format, such as "JPEG" or "PNG".
infobar-file-format = Biçim

# File creation timestamp from image or video embedded metadata.
infobar-originally-created = Orijinal Oluşturulma Tarihi

# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = Orijinal Değiştirilme Tarihi

# Duration (HH:MM) of video.
infobar-video-duration = Süre

# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = Kapsayıcı Biçimi

# Video codec, such as "AV1".
infobar-video-codec = Video Codec Bileşeni

# Audio codec, such as "OPUS".
infobar-audio-codec = Ses Codec Bileşeni

# Width and height of photo or video.
infobar-dimensions = Boyutlar

## Preferences

# Title of preferences dialog
prefs-title = Tercihler

# Title of section of preferences for views
prefs-views-section = Görünümler
  .description = Kenar çubuğu menülerini göster veya gizle

# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Özçekimler
  .subtitle = iOS cihazlarda çekilen özçekimler için ayrı bir görünüm gösterir. Uygulamak için {-app-name} uygulamasını yeniden başlatın.

## Progress bar for background tasks

# Extracting details from photo EXIF data
progress-metadata-photos = Fotoğraf meta verileri işleniyor.

# Extracting details from video container metadata
progress-metadata-videos = Video meta verileri işleniyor.

# Generating thumbnails from photos
progress-thumbnails-photos = Fotoğraf küçük resimleri oluşturuluyor.

# Generating thumbnails from videos
progress-thumbnails-videos = Video küçük resimleri oluşturuluyor.

# Transcoding videos to a compatible format
progress-convert-videos = Videolar dönüştürülüyor.

# Extracting motion photo videos
progress-motion-photo = Hareketli fotoğraflar işleniyor.

# Not doing any background work
progress-idle = Boşta.

## Notification banner for background tasks

# Similar to the progress bar, but allows for longer messages.

# Scanning file system for new photos
banner-scan-photos = Fotoğraflar için dosya sistemi taranıyor.

# Scanning file system for new videos
banner-scan-videos = Videolar için dosya sistemi taranıyor.

# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos = Fotoğraf meta verileri işleniyor.

# Processing new videos to extract metadata from video container.
banner-metadata-videos = Video meta verileri işleniyor.

# Generating thumbnails for all photos.
banner-thumbnails-photos = Fotoğraf küçük resimleri oluşturuluyor. Bu biraz zaman alacaktır.

# Generating thumbnails for all videos.
banner-thumbnails-videos = Video küçük resimleri oluşturuluyor. Bu biraz zaman alacaktır.

# Updating the database to remove details of absent photos.
banner-clean-photos = Fotoğraf veritabanı bakımı yapılıyor.

# Updating the database to remove details of absent videos.
banner-clean-videos = Video veritabanı bakımı yapılıyor.

# Extracting video component from Android motion photos
banner-extract-motion-photos = Hareketli fotoğraflar işleniyor.

## Primary menu

# The "hamburger" menu on the main app navigation sidebar.

# Menu item to show preferences dialog
primary-menu-preferences = Tercihler

# Menu item to show "about" dialog
primary-menu-about = {-app-name} Hakkında
