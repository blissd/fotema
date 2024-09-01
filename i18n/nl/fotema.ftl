# Main Navigation Pages
# Title for video album.
videos-album = Video's
# Main Navigation Pages
# Title for album of iOS live photos and Android motion photos.
animated-album = Bewegend
# Photo/Video Information Sidebar
# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = Containerformaat
# Photo/Video Information Sidebar
# Video codec, such as "AV1".
infobar-video-codec = Videocodec
# Photo/Video Information Sidebar
# Audio codec, such as "OPUS".
infobar-audio-codec = Audiocodec
# Photo/Video Information Sidebar
# Width and height of photo or video.
infobar-dimensions = Afmetingen
# Preferences
# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Selfies
    .subtitle = Toon een aparte sectie voor selfies die op iOS-apparaten gemaakt zijn. Herstart { -app-name } om de wijziging toe te passen.
# Preferences
# Title of section of preferences for views
prefs-views-section = Zijpaneelsecties
    .description = Toon of verberg zijpaneelsecties
# Progress bar for background tasks
# Extracting details from photo EXIF data
progress-metadata-photos = Bezig met verwerken van metagegevens…
# Progress bar for background tasks
# Extracting details from video container metadata
progress-metadata-videos = Bezig met verwerken van metagegevens…
# Progress bar for background tasks
# Generating thumbnails from photos
progress-thumbnails-photos = Bezig met maken van fotominiaturen…
# Progress bar for background tasks
# Generating thumbnails from videos
progress-thumbnails-videos = Bezig met maken van videominiaturen…
# Progress bar for background tasks
# Transcoding videos to a compatible format
progress-convert-videos = Bezig met omzetten…
# Progress bar for background tasks
# Extracting motion photo videos
progress-motion-photo = Bezig met verwerken van actiefoto's…
# Progress bar for background tasks
# Not doing any background work
progress-idle = Inactief.
# Notification banner for background tasks
# Scanning file system for new photos
banner-scan-photos = Bezig met zoeken naar foto's…
# Notification banner for background tasks
# Scanning file system for new videos
banner-scan-videos = Bezig met zoeken naar video's…
# Thumbnail decorations
# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label =
    { $month ->
        [1] januari { $year }
        [2] februari { $year }
        [3] maart { $year }
        [4] april { $year }
        [5] mei { $year }
        [6] Juni { $year }
        [7] juli { $year }
        [8] augustus { $year }
        [9] september { $year }
        [10] oktober { $year }
        [11] november { $year }
        [12] december { $year }
       *[other] { $year }
    }
# About Dialog
# Section header for open source projects acknowledgements.
about-opensource = Opensourceprojecten
# Photo/Video Viewer
# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
    .tooltip = Vorige
# About Dialog
# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits = David Bliss <hello@fotema.app>
# Photo/Video Viewer
# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
    .tooltip = Volgende
# Photo/Video Viewer
# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
    .tooltip = Afspelen/Pauzeren
# Photo/Video Viewer
# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
    .tooltip = 10 seconden vooruitspoelen
# Photo/Video Viewer
# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
    .tooltip = 10 seconden terugspoelen
# Photo/Video Viewer
# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
    .tooltip = Dempen/Ontdempen
# Photo/Video Viewer
# Convert all incompatible videos description.
viewer-convert-all-description = Deze video moet worden omgezet voordat afspelen mogelijk is. Dit proces is eenmalig, maar kan wel enige tijd duren.
# Photo/Video Viewer
# Viewer failed to load an image or video.
viewer-error-failed-to-load = Laden mislukt
# Photo/Video Viewer
# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file =
    Het bestand ontbreekt en kan daarom niet worden getoond:
    { $file_name }
# Photo/Video Viewer
# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = De bestandslocatie is niet aangetroffen in de databank
# Photo/Video Information Sidebar
# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = Map
    .tooltip = Bijbehorende map openen
# Terms
-app-name = Fotema
# Notification banner for background tasks
# Generating thumbnails for all photos.
banner-thumbnails-photos = Bezig met maken van fotominiaturen… Dit kan even duren.
# Notification banner for background tasks
# Generating thumbnails for all videos.
banner-thumbnails-videos = Bezig met maken van videominiaturen… Dit kan even duren.
# Notification banner for background tasks
# Updating the database to remove details of absent photos.
banner-clean-photos = Fotodatabankonderhoud.
# Notification banner for background tasks
# Updating the database to remove details of absent videos.
banner-clean-videos = Videodatabankonderhoud.
# Notification banner for background tasks
# Extracting video component from Android motion photos
banner-extract-motion-photos = Bezig met verwerken van actiefoto's…
# Primary menu
# Menu item to show preferences dialog
primary-menu-preferences = Voorkeuren
# Primary menu
# Menu item to show "about" dialog
primary-menu-about = Over { -app-name }
# Main Navigation Pages
# Title for library page, which contains the "all", "months", and "years" pages.
library-page = Bibliotheek
# Main Navigation Pages
# Title for months album.
months-album = Maand
# Main Navigation Pages
# Title for all photos/videos album.
all-album = Dag
# Main Navigation Pages
# Title for album of selfies.
selfies-album = Selfies
# Main Navigation Pages
# Title for years album.
years-album = Jaar
# Main Navigation Pages
# Title for album showing all folders.
folders-album = Mappen
# Main Navigation Pages
# Title for album showing contents of one folder.
folder-album = Map
# Main Navigation Pages
# Title for places page which shows photos overlayed onto a map.
places-page = Locaties
# Photo/Video Viewer
# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = Eigenschappen tonen
# Photo/Video Information Sidebar
# Duration (HH:MM) of video.
infobar-video-duration = Duur
# Photo/Video Information Sidebar
# File format, such as "JPEG" or "PNG".
infobar-file-format = Bestandsformaat
# Photo/Video Information Sidebar
# File creation timestamp from file system metadata.
infobar-file-created = Aanmaakdatum
# Photo/Video Information Sidebar
# File creation timestamp from image or video embedded metadata.
infobar-originally-created = Gemaakt op
# Preferences
# Title of preferences dialog
prefs-title = Voorkeuren
# Photo/Video Viewer
# Button to convert all incompatible videos.
viewer-convert-all-button = Alle incompatibele video's omzetten
# Photo/Video Information Sidebar
# File name of photo or video
infobar-file-name = Bestandsnaam
# Photo/Video Information Sidebar
# File modification timestamp from file system metadata.
infobar-file-modified = Bewerkdatum
# Photo/Video Information Sidebar
# File size file system metadata.
infobar-file-size = Bestandsgrootte
# Photo/Video Information Sidebar
# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = Bewerkt op
# Notification banner for background tasks
# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos = Bezig met verwerken van metagegevens…
# Notification banner for background tasks
# Processing new videos to extract metadata from video container.
banner-metadata-videos = Bezig met verwerken van metagegevens…
