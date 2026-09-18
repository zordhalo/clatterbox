# Credits — Tactile

All files in this pack are CC0 (public domain), sourced from Freesound. License confirmed per
sound on each source page (`creativecommons.org/publicdomain/zero/1.0/`) — Freesound packs can
mix licenses, so each file's page was checked individually, not just the pack page.

| Shipped as | Original file | Author | Source |
|---|---|---|---|
| down-01.mp3 | 766628.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766628/ |
| down-02.mp3 | 766629.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766629/ |
| down-03.mp3 | 766630.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766630/ |
| down-04.mp3 | 766631.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766631/ |
| down-05.mp3 | 766632.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766632/ |
| down-06.mp3 | 766633.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766633/ |
| down-07.mp3 | 766634.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766634/ |
| down-08.mp3 | 766635.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766635/ |
| down-09.mp3 | 766637.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766637/ |
| down-10.mp3 | 766638.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766638/ |
| down-11.mp3 | 766639.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766639/ |
| down-12.mp3 | 766640.mp3 | StavSounds | https://freesound.org/people/StavSounds/sounds/766640/ |
| up-01.mp3 | 570755_key-release.mp3 | Foxfire- | https://freesound.org/people/Foxfire-/sounds/570755/ |
| enter-down-01.mp3 | 627647_enter-key.mp3 | alpinemesh | https://freesound.org/people/alpinemesh/sounds/627647/ |
| backspace-down-01.mp3 | 380141_single-key.mp3 | yottasounds | https://freesound.org/people/yottasounds/sounds/380141/ |

Pack license: **CC0-1.0** for every file above. Files are the Freesound HQ preview downloads
(near-original quality; full originals require a Freesound account login to fetch), byte-identical
to the copies under `research/raw-sounds/`. No re-encoding was done.

`enter.gain_db` (-2.0 dB) and `backspace.gain_db` (+1.0 dB) trim the alpinemesh (Corsair K70 RGB)
and yottasounds takes to sit level with the StavSounds `default.down` set, since they were
recorded on different boards/mics. These are a first-pass estimate — nobody has yet A/B'd them
on real speakers; if a level mismatch is heard, tune the numbers here and open a PR.

Space and modifier classes are not represented by real recordings in this pack; they are derived
at load time from `default.down` (see the "Sound pack format" section of the top-level README).
