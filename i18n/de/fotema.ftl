# SPDX-FileCopyrightText: © 2024 David Bliss
#
# SPDX-License-Identifier: GPL-3.0-or-later

## Terms

# See https://projectfluent.org/fluent/guide/terms.html

-app-name = Fotema

## Main Navigation Pages

# Title for library page, which contains the "all", "months", and "years" pages.
library-page = Bibliothek

# Title for years album.
years-album = Jahr

# Title for months album.
months-album = Monat

# Title for all photos/videos album.
all-album = Tag

# Title for video album.
videos-album = Videos

# Title for album of selfies.
selfies-album = Selfies

# Title for album of iOS live photos and Android motion photos.
animated-album = Animiert

# Title for album showing all folders.
folders-album = Alle Alben

# Title for album showing contents of one folder.
folder-album = Album

# Title for places page which shows photos overlayed onto a map.
places-page = Orte

## Thumbnail decorations

# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label = { $month ->
   [1] Januar {$year}
   [2] Februar {$year}
   [3] März {$year}
   [4] April {$year}
   [5] Mai {$year}
   [6] Juni {$year}
   [7] Juli {$year}
   [8] August {$year}
   [9] September {$year}
   [10] Oktober {$year}
   [11] November {$year}
   [12] Dezember {$year}
  *[other] {$year}
}

## About Dialog

# Section header for open source projects acknowledgements.
about-opensource = Quelloffene Projekte

# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits =
  David Bliss <hello@fotema.app>

## Photo/Video Viewer

# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = Eigenschaften anzeigen

# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
  .tooltip = Weiter

# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
  .tooltip = Zurück

# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
  .tooltip = Fortsetzen/Pausieren

# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
  .tooltip = 10 Sekunden vorwärts springen

# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
  .tooltip = 10 Sekunden rückwärts springen

# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
  .tooltip = Stumm/Laut

# Convert all incompatible videos description.
viewer-convert-all-description = Dieses Video muss konvertiert werden, bevor es abgespielt werden kann. Dies muss nur einmal geschehen, aber es dauert, das Video zu konvertieren.

# Button to convert all incompatible videos.
viewer-convert-all-button = Alle inkompatiblen Videos konvertieren

# Viewer failed to load an image or video.
viewer-error-failed-to-load = Laden fehlgeschlagen

# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file = Die Datei kann nicht angezeigt werden, weil sie fehlt:
  {$file_name}

# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = Dateipfad nicht in der Datenbank vorhanden

## Photo/Video Information Sidebar

# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = Ordner
  .tooltip = Beinhaltenden Ordner öffnen

# File name of photo or video
infobar-file-name = Dateiname

# File creation timestamp from file system metadata.
infobar-file-created = Datei erstellt

# File modification timestamp from file system metadata.
infobar-file-modified = Datei verändert

# File size file system metadata.
infobar-file-size = Dateigröße

# File format, such as "JPEG" or "PNG".
infobar-file-format = Bildformat

# File creation timestamp from image or video embedded metadata.
infobar-originally-created = Original erstellt

# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = Original verändert

# Duration (HH:MM) of video.
infobar-video-duration = Dauer

# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = Container-Format

# Video codec, such as "AV1".
infobar-video-codec = Video Codec

# Audio codec, such as "OPUS".
infobar-audio-codec = Audio Codec

# Width and height of photo or video.
infobar-dimensions = Bildgröße

## Preferences

# Title of preferences dialog
prefs-title = Einstellungen

# Title of section of preferences for views
prefs-views-section = Ansicht
  .description = Seitenleiste anzeigen/verstecken

# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Selfies
  .subtitle = Zeige eine separate Ansicht für Selfies, die mit iOS-Geräten aufgenommen wurden. Starte {-app-name} neu, um es anzuwenden.

## Progress bar for background tasks

# Extracting details from photo EXIF data
progress-metadata-photos = Foto-Metadaten werden verarbeitet.

# Extracting details from video container metadata
progress-metadata-videos = Video-Metadaten werden verarbeitet.

# Generating thumbnails from photos
progress-thumbnails-photos = Foto-Miniaturansichten werden erstellt.

# Generating thumbnails from videos
progress-thumbnails-videos = Video-Miniaturansichten werden erstellt.

# Transcoding videos to a compatible format
progress-convert-videos = Videos werden konvertiert.

# Extracting motion photo videos
progress-motion-photo = Bewegungsfotos werden verarbeitet.

# Not doing any background work
progress-idle = Nichts zu tun.

## Notification banner for background tasks

# Similar to the progress bar, but allows for longer messages.

# Scanning file system for new photos
banner-scan-photos = Dateien werden nach Fotos durchsucht.

# Scanning file system for new videos
banner-scan-videos = Dateien werden nach Videos durchsucht.

# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos = Foto-Metadaten werden verarbeitet.

# Processing new videos to extract metadata from video container.
banner-metadata-videos = Video-Metadaten werden erstellt.

# Generating thumbnails for all photos.
banner-thumbnails-photos = Foto-Miniaturansichten werden erstellt. Dies kann eine Weile dauern.

# Generating thumbnails for all videos.
banner-thumbnails-videos = Video-Miniaturansichten werden erstellt. Dies kann eine Weile dauern.

# Updating the database to remove details of absent photos.
banner-clean-photos = Fotodatenbank wird optimiert

# Updating the database to remove details of absent videos.
banner-clean-videos = Videodatenbank wird optimiert

# Extracting video component from Android motion photos
banner-extract-motion-photos = Bewegungsfotos werden verarbeitet

## Primary menu

# The "hamburger" menu on the main app navigation sidebar.

# Menu item to show preferences dialog
primary-menu-preferences = Einstellungen

# Menu item to show "about" dialog
primary-menu-about = Info zu {-app-name}
