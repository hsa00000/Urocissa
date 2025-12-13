migration 注意：
大部分的 field 都是直觀上的遷移
但是以下比較特別的 field 或是 value 要注意

關於 description

1. abstractData.album.userDefinedMetadata.\_user_defined_description?.[0] -> object.description
2. abstractData.database.exif_vec.\_user_defined_description -> object.description
