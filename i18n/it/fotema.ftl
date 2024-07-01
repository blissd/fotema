## Terms

# See https://projectfluent.org/fluent/guide/terms.html

-app-name = Fotema

## Main Navigation Pages

# Title for library page, which contains the "all", "months", and "years" pages.
library-page = Libreria

# Title for years album.
years-album = Anno

# Title for months album.
months-album = Mese

# Title for all photos/videos album.
all-album = Giorno

# Title for video album.
videos-album = Video

# Title for album of selfies.
selfies-album = Selfie

# Title for album of iOS live photos and Android motion photos.
animated-album = Animato

# Title for album showing all folders.
folders-album = Cartelle

# Title for album showing contents of one folder.
folder-album = Cartella

## Thumbnail decorations

# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label = { $month ->
   [1] Gennaio {$year}
   [2] Febbraio {$year}
   [3] Marzo {$year}
   [4] Aprile {$year}
   [5] Maggio {$year}
   [6] Giugno {$year}
   [7] Luglio {$year}
   [8] Agosto {$year}
   [9] Settembre {$year}
   [10] Ottobre {$year}
   [11] Novembre {$year}
   [12] Dicembre {$year}
  *[other] {$year}
}

## About Dialog

# Section header for open source projects acknowledgements.
about-opensource = Progetti Open Source

# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits =
  Albano Battistella <albanobattistella@gmail.com>

## Photo/Video Viewer

# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = Mostra proprietà

# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
  .tooltip = Prossima

# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
  .tooltip = Precedente

# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
  .tooltip = Avvia/Pausa

# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
  .tooltip = Vai avanti di 10 secondi

# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
  .tooltip = Vai indietro di 10 secondi

# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
  .tooltip = Muto/Smuta

# Convert all incompatible videos description
viewer-convert-all-description = Questo video deve essere convertito prima di poter essere riprodotto. Questo deve accadere solo una volta, ma ci vuole un po’ di tempo per convertire un video.

viewer-convert-all-button = Converti tutti i video incompatibili

## Photo/Video Information Sidebar

# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = Cartella
  .tooltip = Apri contenuto della cartella

# File name of photo or video
infobar-file-name = Nome file

# File creation timestamp from file system metadata.
infobar-file-created = File creato

# File modification timestamp from file system metadata.
infobar-file-modified = File modificato

# File size file system metadata.
infobar-file-size = Dimensione del file

# File format, such as "JPEG" or "PNG".
infobar-file-format = Formato

# File creation timestamp from image or video embedded metadata.
infobar-originally-created = Originariamente creato

# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = Originariamente modificato

# Duration (HH:MM) of video.
infobar-video-duration = Durata

# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = Formato del contenitore

# Video codec, such as "AV1".
infobar-video-codec = Codec Video

# Audio codec, such as "OPUS".
infobar-audio-codec = Codec Audio

# Width and height of photo or video.
infobar-dimensions = Dimensioni

## Preferences

# Title of section of preferences for views
prefs-views-section = Viste
  .description = Mostra o nascondi le visualizzazioni della barra laterale

# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Selfie
  .subtitle = Mostra una vista separata per i selfie scattati su dispositivi iOS. Riavvia {-app-name} per applicare.

## Progress bar for background tasks

# Extracting details from photo EXIF data
progress-metadata-photos = Elaborazione dei metadati delle foto.

# Extracting details from video container metadata
progress-metadata-videos = Elaborazione dei metadati video.

# Generating thumbnails from photos
progress-thumbnails-photos = Generazione di miniature di foto.

# Generating thumbnails from videos
progress-thumbnails-videos = Generazione di miniature dei video.

# Transcoding videos to a compatible format
progress-convert-videos = Conversione di video.

# Not doing any background work
progress-idle = Inattivo.

## Notification banner for background tasks

# Similar to the progress bar, but allows for longer messages.

# Scanning file system for new photos
banner-scan-photos = Scansione del file system per le foto.

# Scanning file system for new videos
banner-scan-videos = Scansione del file system per i video.

# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos =Elaborazione dei metadati delle foto.

# Processing new videos to extract metadata from video container.
banner-metadata-videos = Elaborazione dei metadati video.

# Generating thumbnails for all photos.
banner-thumbnails-photos = Generazione di miniature di foto. Ci vorrà un po’ di tempo.

# Generating thumbnails for all videos.
banner-thumbnails-videos = Generazione di miniature dei video. Ci vorrà un po’ di tempo.

# Updating the database to remove details of absent photos.
banner-clean-photos = Manutenzione del database fotografico.

# Updating the database to remove details of absent videos.
banner-clean-videos = Manutenzione del database video.

## Primary menu

# The "hamburger" menu on the main app navigation sidebar.

# Menu item to show preferences dialog
primary-menu-preferences = Preferenze

# Menu item to show "about" dialog
primary-menu-about = Informazioni su {-app-name}
