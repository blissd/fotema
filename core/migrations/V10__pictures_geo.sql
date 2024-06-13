-- GPS data extracted from exif tags
CREATE TABLE pictures_geo (
        picture_id         INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for picture
        longitude          REAL NOT NULL, -- decimal longitude
        latitude           REAL NOT NULL, -- decimal latitude
        h3_r3_id           INTEGER NOT NULL, -- H3 index ID at resolution 3
        h3_r4_id           INTEGER NOT NULL, -- H3 index ID at resolution 4
        h3_r5_id           INTEGER NOT NULL, -- H3 index ID at resolution 5
        h3_r6_id           INTEGER NOT NULL, -- H3 index ID at resolution 6
        h3_r7_id           INTEGER NOT NULL, -- H3 index ID at resolution 7
        h3_r8_id           INTEGER NOT NULL, -- H3 index ID at resolution 8
        FOREIGN KEY (picture_id) REFERENCES pictures (picture_id) ON DELETE CASCADE
);
