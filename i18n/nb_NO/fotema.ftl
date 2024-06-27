# Photo/Video Information Sidebar
# File size file system metadata.
infobar-file-size = Filstørrelse
# Main Navigation Pages
# Title for video album.
videos-album = Videoer
# Main Navigation Pages
# Title for album showing all folders.
folders-album = Mapper
# Main Navigation Pages
# Title for album of selfies.
selfies-album = Selvportrett
# Main Navigation Pages
# Title for album of iOS live photos and Android motion photos.
animated-album = Animert
# Photo/Video Viewer
# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
    .tooltip = Hopp bakover 10 sekunder
# Photo/Video Viewer
# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
    .tooltip = Forstum/opphev forstumming
# Photo/Video Viewer
# Convert all incompatible videos description.
viewer-convert-all-description = Videoen må konverteres før den kan spilles. Det tar sin tid, men er kun én gang.
# Photo/Video Viewer
# Button to convert all incompatible videos.
viewer-convert-all-button = Konverter alle ukompatible videoer
# Photo/Video Information Sidebar
# File creation timestamp from file system metadata.
infobar-file-created = Fil opprettet
# Photo/Video Information Sidebar
# File format, such as "JPEG" or "PNG".
infobar-file-format = Format
# Photo/Video Information Sidebar
# File creation timestamp from image or video embedded metadata.
infobar-originally-created = Opprinnelig opprettet
# Photo/Video Information Sidebar
# Width and height of photo or video.
infobar-dimensions = Dimensjoner
# Main Navigation Pages
# Title for years album.
years-album = År
# Main Navigation Pages
# Title for all photos/videos album.
all-album = Dag
# Terms
-app-name = Fotema
# Main Navigation Pages
# Title for library page, which contains the "all", "months", and "years" pages.
library-page = Bibliotek
# Main Navigation Pages
# Title for months album.
months-album = Måned
# Main Navigation Pages
# Title for album showing contents of one folder.
folder-album = Mappe
# Photo/Video Information Sidebar
# File name of photo or video
infobar-file-name = Filnavn
# Photo/Video Information Sidebar
# File modification timestamp from file system metadata.
infobar-file-modified = Fil endret
# Primary menu
# Menu item to show preferences dialog
primary-menu-preferences = Innstillinger
# Primary menu
# Menu item to show "about" dialog
primary-menu-about = Om { -app-name }
# Photo/Video Information Sidebar
# Duration (HH:MM) of video.
infobar-video-duration = Varighet
# Photo/Video Information Sidebar
# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = Beholder-format
# Photo/Video Information Sidebar
# Video codec, such as "AV1".
infobar-video-codec = Videokodek
# Photo/Video Information Sidebar
# Audio codec, such as "OPUS".
infobar-audio-codec = Lydkodek
# About Dialog
# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits = Allan Nordhøy <epost@anotheragency.no>
# About Dialog
# Section header for open source projects acknowledgements.
about-opensource = Frie prosjekter
# Photo/Video Viewer
# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = Vis egenskaper
# Photo/Video Viewer
# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
    .tooltip = Neste
# Photo/Video Viewer
# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
    .tooltip = Forrige
# Photo/Video Viewer
# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
    .tooltip = Spill/pause
# Photo/Video Viewer
# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
    .tooltip = Hopp forover 10 sekunder
# Photo/Video Information Sidebar
# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = Opprinnelig endret
# Progress bar for background tasks
# Extracting details from photo EXIF data
progress-metadata-photos = Behandler bildemetadata …
# Progress bar for background tasks
# Not doing any background work
progress-idle = Ledig.
# Notification banner for background tasks
# Updating the database to remove details of absent videos.
banner-clean-videos = Vedlikehold av videodatabase.
# Notification banner for background tasks
# Extracting video component from Android motion photos
banner-extract-motion-photos = Pakker ut bevegelige bilder.
# Photo/Video Viewer
# Viewer failed to load an image or video.
viewer-error-failed-to-load = Klarte ikke å laste inn
# Photo/Video Viewer
# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file =
    Kan ikke vise manglende fil:
    { $file_name }
# Photo/Video Viewer
# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = Filstien finnes ikke i databasen
# Thumbnail decorations
# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label =
    { $month ->
        [1] Januar { $year }
        [2] Februar { $year }
        [3] Mars{ $year }
        [4] April { $year }
        [5] Mai { $year }
        [6] Juni { $year }
        [7] Juli { $year }
        [8] August { $year }
        [9] September { $year }
        [10] Oktober { $year }
        [11] November { $year }
        [12] Desember { $year }
       *[other] { $year }
    }
# Photo/Video Information Sidebar
# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = Mappe
    .tooltip = Åpne inneholdende mappe
# Preferences
# Title of section of preferences for views
prefs-views-section = Visninger
    .description = Vis eller skjul sidefeltsvisninger
# Preferences
# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Selvportretter
    .subtitle = Viser en egen visning for selvportrett tatt på iOS-enheter. Start  { -app-name } igjen for å bruke.
# Progress bar for background tasks
# Extracting details from video container metadata
progress-metadata-videos = Behandler videometadata …
# Progress bar for background tasks
# Generating thumbnails from videos
progress-thumbnails-videos = Genererer video-miniatyrbilder …
# Notification banner for background tasks
# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos = Behandler bildemetadata …
# Progress bar for background tasks
# Generating thumbnails from photos
progress-thumbnails-photos = Genererer miniatyrbilder …
# Progress bar for background tasks
# Transcoding videos to a compatible format
progress-convert-videos = Konverterer videoer …
# Notification banner for background tasks
# Scanning file system for new photos
banner-scan-photos = Skanner filsystem for bilder …
# Progress bar for background tasks
# Extracting motion photo videos
progress-motion-photo = Behandler bevegelige bilder …
# Notification banner for background tasks
# Scanning file system for new videos
banner-scan-videos = Skanner filsystem for videoer …
# Notification banner for background tasks
# Processing new videos to extract metadata from video container.
banner-metadata-videos = Behandler videometadata …
# Notification banner for background tasks
# Generating thumbnails for all photos.
banner-thumbnails-photos = Generer miniatyrbilder … Dette kan ta en stund.
# Notification banner for background tasks
# Generating thumbnails for all videos.
banner-thumbnails-videos = Generer video-miniatyrbilder … Dette kan ta en stund.
# Notification banner for background tasks
# Updating the database to remove details of absent photos.
banner-clean-photos = Vedlikehold av bildedatabase.
