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
month-thumbnail-label =
    { $month ->
        [1] Gennaio { $year }
        [2] Febbraio { $year }
        [3] Marzo { $year }
        [4] Aprile { $year }
        [5] Maggio { $year }
        [6] Giugno { $year }
        [7] Luglio { $year }
        [8] Agosto { $year }
        [9] Settembre { $year }
        [10] Ottobre { $year }
        [11] Novembre { $year }
        [12] Dicembre { $year }
       *[other] { $year }
    }

## About Dialog

# Section header for open source projects acknowledgements.
about-opensource = Progetti Open Source
# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits = Albano Battistella <albanobattistella@gmail.com>

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
    .subtitle = Mostra una vista separata per i selfie scattati su dispositivi iOS. Riavvia { -app-name } per applicare.

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
banner-metadata-photos = Elaborazione dei metadati delle foto.
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
primary-menu-about = Informazioni su { -app-name }
# Main Navigation Pages
# Title for places page which shows photos overlayed onto a map.
places-page = Luoghi
# Photo/Video Viewer
# Viewer failed to load an image or video.
viewer-error-failed-to-load = Caricamento non riuscito
# Photo/Video Viewer
# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = Percorso del file non presente nel database
# Photo/Video Viewer
# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file =
    Impossibile visualizzare il file perché manca:
    { $file_name }
# Progress bar for background tasks
# Extracting motion photo videos
progress-motion-photo = Elaborazione di foto in movimento.
# Notification banner for background tasks
# Extracting video component from Android motion photos
banner-extract-motion-photos = Elaborazione di foto in movimento.
# Preferences
# Title of preferences dialog
prefs-title = Preferenze
people-set-face-thumbnail = Usa come miniatura
prefs-views-faces = Rilevamento del volto
    .subtitle = Abilita il rilevamento del volto quando Fotema si avvia. Questo è un processo che richiede molto tempo.
people-person-search =
    .placeholder = Nome della persona
people-face-ignore = Ignora
people-not-this-person = Non { $name }
progress-detect-faces-photos = Rilevamento dei volti nelle foto.
banner-detect-faces-photos = Rilevamento dei volti nelle foto. Ci vorrà un po' di tempo.
progress-recognize-faces-photos = Riconoscere le persone nelle foto.
person-rename-dialog =
    .heading = Rinomina persona?
    .placeholder = Nuovo nome
    .cancel-button = Annulla
    .rename-button = Rinomina
person-menu-delete = Elimina persona
people-page = Persone
person-menu-rename = Rinomina persona
people-page-status-off =
    .title = Abilitare il rilevamento dei volti?
    .description = { -app-name } può rilevare automaticamente i volti e riconoscere le persone, ma è un processo che richiede molto tempo. Vuoi abilitare questa funzionalità?
    .notice = { -app-name } deve scaricare circa 45 megabyte di dati per riconoscere volti e persone.
    .enable = Abilita
people-page-status-no-people =
    .title = Nessuna persona trovata
    .description =
        { -app-name } cercherà i volti nelle nuove foto quando verrà avviata.
        Assegna un nome alle persone nelle tue foto in modo che { -app-name } possa creare un album per ciascuna persona.
viewer-faces-menu =
    .tooltip = Menu volti
    .restore-ignored = Ripristina tutti i volti ignorati
    .ignore-unknown = Ignora tutti i volti sconosciuti
    .scan = Cerca altri volti
people-set-name = Imposta nome
banner-recognize-faces-photos = Riconoscere le persone nelle foto. Ci vorrà un po' di tempo.
person-delete-dialog =
    .heading = Eliminare la persona?
    .body = Nessuna immagine o video verrà eliminato.
    .cancel-button = Annulla
    .delete-button = Elimina
prefs-ui-chronological-album-sort = Ordinamento
    .subtitle = Ordinamento cronologico per gli album.
    .ascending = Crescente
    .descending = Decrescente
prefs-machine-learning-section = Machine Learning
    .description = Configura le funzionalità di apprendimento automatico.
prefs-machine-learning-face-detection = Rilevamento del volto
    .subtitle = Abilita il rilevamento del volto quando { -app-name } si avvia. Questo è un processo che richiede molto tempo.
prefs-ui-section = UI
    .description = Personalizza l'interfaccia utente.
prefs-ui-selfies = Selfies
    .subtitle = Visualizza un album separato per i selfie scattati su un device iOS. Riavvia { -app-name } per applicare.
prefs-library-section =
    .title = Libreria
    .description =
        Configura la directory della libreria.
        Attenzione: la modifica della directory delle immagini può causare la rielaborazione di tutte le immagini da parte di { -app-name }.
prefs-library-section-pictures-dir =
    .title = Directory delle immagini
    .tooltip = Scegli la directory delle immagini.
onboard-select-pictures =
    .title = Benvenuti in { -app-name }.
    .description =
        Seleziona la directory in cui tieni la tua libreria di immagini.

        Se hai utilizzato una versione precedente di { -app-name } in cui la tua libreria di immagini veniva rilevata automaticamente, seleziona la stessa directory qui per evitare qualsiasi elaborazione duplicata delle immagini.
    .button = Seleziona directory
banner-convert-videos = Conversione video.
banner-button-stop =
    .label = Interrompere
    .tooltip = Interrompi tutte le attività in background.
banner-stopping = Interruzione delle attività...
prefs-albums-chronological-sort = Ordine di ordinamento
    .subtitle = Ordine cronologico degli album.
    .ascending = Crescente
    .descending = Decrescente
prefs-processing-section = Elaborazione di foto e video
    .description = Configura le funzionalità di elaborazione di foto e video.
prefs-processing-face-detection = Rilevamento dei volti
    .subtitle = Rileva i volti e riconosci le persone a cui hai dato un nome. È un processo che richiede tempo.
prefs-albums-section = Album
    .description = Configura gli album.
prefs-albums-selfies = Selfie
    .subtitle = Mostra un album separato per i selfie scattati su dispositivi iOS. Riavvia { -app-name } per applicare.
prefs-processing-motion-photos = Foto in movimento
    .subtitle = Rileva le foto in movimento di Android ed estrai i video.
