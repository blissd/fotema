# Progress bar for background tasks
# Extracting motion photo videos
progress-motion-photo = Memproses motion photo.
# Main Navigation Pages
# Title for years album.
years-album = Tahun
# Main Navigation Pages
# Title for all photos/videos album.
all-album = Hari
# Main Navigation Pages
# Title for video album.
videos-album = Video
# Main Navigation Pages
# Title for album of selfies.
selfies-album = Swafoto
# Main Navigation Pages
# Title for album showing all folders.
folders-album = Folder
# Main Navigation Pages
# Title for album showing contents of one folder.
folder-album = Folder
# About Dialog
# Section header for open source projects acknowledgements.
about-opensource = Proyek Sumber Terbuka
# About Dialog
# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits = Christian Elbrianno
# Photo/Video Viewer
# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
    .tooltip = Lanjut
# Photo/Video Viewer
# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
    .tooltip = Putar/Jeda
# Photo/Video Viewer
# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
    .tooltip = Maju 10 Detik
# Photo/Video Viewer
# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
    .tooltip = Kembali 10 Detik
# Photo/Video Viewer
# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = Jalur berkas tidak ada dalam database
# Photo/Video Information Sidebar
# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = Folder
    .tooltip = Buka Folder Terkait
# Photo/Video Information Sidebar
# File size file system metadata.
infobar-file-size = Ukuran Berkas
# Photo/Video Information Sidebar
# File format, such as "JPEG" or "PNG".
infobar-file-format = Format
# Photo/Video Information Sidebar
# File creation timestamp from image or video embedded metadata.
infobar-originally-created = Tanggal Pembuatan Asli
# Photo/Video Information Sidebar
# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = Format Kontainer
# Photo/Video Information Sidebar
# Video codec, such as "AV1".
infobar-video-codec = Video Codec
# Photo/Video Information Sidebar
# Audio codec, such as "OPUS".
infobar-audio-codec = Audio Codec
# Preferences
# Title of preferences dialog
prefs-title = Preferensi
# Progress bar for background tasks
# Extracting details from photo EXIF data
progress-metadata-photos = Memproses metadata foto.
# Progress bar for background tasks
# Extracting details from video container metadata
progress-metadata-videos = Memproses metadata video.
# Progress bar for background tasks
# Generating thumbnails from photos
progress-thumbnails-photos = Membuat gambar mini foto.
# Progress bar for background tasks
# Generating thumbnails from videos
progress-thumbnails-videos = Membuat gambar mini video.
# Progress bar for background tasks
# Not doing any background work
progress-idle = Tidak aktif.
# Notification banner for background tasks
# Scanning file system for new photos
banner-scan-photos = Memindai berkas sistem untuk foto.
# Notification banner for background tasks
# Scanning file system for new videos
banner-scan-videos = Memindai berkas sistem untuk video.
# Notification banner for background tasks
# Processing new videos to extract metadata from video container.
banner-metadata-videos = Memproses metadata video.
# Notification banner for background tasks
# Generating thumbnails for all photos.
banner-thumbnails-photos = Membuat gambar mini foto. Ini akan memakan waktu cukup lama.
# Notification banner for background tasks
# Generating thumbnails for all videos.
banner-thumbnails-videos = Membuat gambar mini video. Ini akan memakan waktu cukup lama.
# Notification banner for background tasks
# Extracting video component from Android motion photos
banner-extract-motion-photos = Memproses motion photo.
# Primary menu
# Menu item to show preferences dialog
primary-menu-preferences = Preferensi
# Primary menu
# Menu item to show "about" dialog
primary-menu-about = Tentang { -app-name }
# Terms
-app-name = Fotema
# Main Navigation Pages
# Title for places page which shows photos overlayed onto a map.
places-page = Lokasi
# Main Navigation Pages
# Title for library page, which contains the "all", "months", and "years" pages.
library-page = Pustaka
# Main Navigation Pages
# Title for months album.
months-album = Bulan
# Thumbnail decorations
# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label =
    { $month ->
        [1] Januari { $year }
        [2] Februari { $year }
        [3] Maret { $year }
        [4] April { $year }
        [5] Mei { $year }
        [6] Juni { $year }
        [7] Juli { $year }
        [8] Agustus { $year }
        [9] September { $year }
        [10] Oktober { $year }
        [11] November { $year }
        [12] Desember { $year }
       *[other] { $year }
    }
# Photo/Video Viewer
# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = Tampilkan properti
# Photo/Video Viewer
# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
    .tooltip = Sebelum
# Photo/Video Viewer
# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
    .tooltip = Bunyikan/Bisukan
# Photo/Video Viewer
# Convert all incompatible videos description.
viewer-convert-all-description = Video ini harus dikonversi sebelum dapat diputar. Hal ini hanya perlu dilakukan sekali, tetapi memerlukan waktu untuk mengonversi video.
# Photo/Video Viewer
# Viewer failed to load an image or video.
viewer-error-failed-to-load = Gagal memuat
# Photo/Video Viewer
# Button to convert all incompatible videos.
viewer-convert-all-button = Konversi semua video yang tidak kompatibel
# Photo/Video Viewer
# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file =
    Tidak dapat menampilkan berkas karena tidak ditemukan:
    { $file_name }
# Photo/Video Information Sidebar
# File name of photo or video
infobar-file-name = Nama Berkas
# Photo/Video Information Sidebar
# File creation timestamp from file system metadata.
infobar-file-created = Tanggal Pembuatan
# Photo/Video Information Sidebar
# File modification timestamp from file system metadata.
infobar-file-modified = Tanggal Modifikasi
# Photo/Video Information Sidebar
# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = Tanggal Modifikasi Asli
# Photo/Video Information Sidebar
# Duration (HH:MM) of video.
infobar-video-duration = Durasi
# Photo/Video Information Sidebar
# Width and height of photo or video.
infobar-dimensions = Dimensi
# Preferences
# Title of section of preferences for views
prefs-views-section = Pratinjau
    .description = Tampilkan atau sembunyikan pratinjau samping
# Preferences
# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Swafoto
    .subtitle = Tampilkan pratinjau khusus untuk swafoto dari perangkat iOS. Mulai ulang { -app-name } untuk menerapkan.
# Progress bar for background tasks
# Transcoding videos to a compatible format
progress-convert-videos = Mengkonversi video.
# Notification banner for background tasks
# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos = Memproses metadata foto.
# Notification banner for background tasks
# Updating the database to remove details of absent photos.
banner-clean-photos = Pemeliharaan database foto.
# Notification banner for background tasks
# Updating the database to remove details of absent videos.
banner-clean-videos = Pemeliharaan database video.
