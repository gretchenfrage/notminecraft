
# Not Minecraft Beta 1.0.2

See http://phoenixkahlo.com/hubs/notminecraftbeta102

To run, `cargo run`. Is simple :)

## Things to it

|thing|to it|
|---|---|
|graphics|core systems good, need enhancements|
|physics|core systems good, but FP precision-related problems|
|blocks|core systems good|
|GUI|core systems good|
|items|not started|
|mobs|not started|
|sound|not started|
|save file|not started|
|multiplayer|not started|
|interpolation|not started|
|game content|few things added|

## More detail

- ab_glyph fork is fine.
- opentype437 is fine.
- mesh_data is good. testing would be quite nice though lol.
- chunk_data is good, add to as needed.
- graphics is decent, could use a couple major enhancements / cleanups:
    - should remove GpuImage, and just have everything always use
      GpuImageArray. this would facilitate particles, which is a major
      low-hanging enhancement
    - fog + skybox shading are major low-hanging enhancements
    - figuring out what's going on with matrix handedness or whatever so I
      can get backface culling working is important
    - there's that very boring bug with text spans of different colors/fonts
    - overall there's like, commented out blocks of code, under-documented
      parts, etc
    - also that slight requesting of additional bind groups or something is
      gonna bother me
    - and eventually the use of push constants could be a nice optimization
- in terms of the minecraft package
    - asset contains some good stuff but maybe should be refactored a bit?
    - sound should be more factored out into its own system, and we need to
      finish making the sound asset acquisition system
    - also should have a system which allows like using resource packs other
      than the default and layering them
    - and also make sure we have proper support for different texture
      resolutions
    - game_data is good but door stuff is unimplemented
    - gui is great!! but there's a couple gui blocks in main_menu that should
      be factored into general purpose GUI block whatevers
    - idk what's going on with util, I think some modules may have been legit
      unplugged?
    - main_menu so far is pretty good
    - ChunkMesh is like a fine little abstraction
    - singleplayer has some good stuff so far
        - block_update_queue is great
        - chunk_loader is great
        - we should like _finish_ unifying the geometry logic factor with physics
          or whatever and also make that play well with tile_meshing
        - movement.rs is a partially-unplugged mess but luckily it's not that
          complicated just in an absolute sense
        - blocks is like fine so far I mean there's like not much there
        - the primary module is rather a mess

Next steps should be:
- sound acquisition
- geometry logic unification
- make prerequsite logic for working particles I thinks
