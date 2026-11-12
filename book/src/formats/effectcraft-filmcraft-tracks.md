# `EffectCraft` & `FilmCraft` `V1`–`V4` Tracks

## Puppet Controls for Facial Expressions & Mouth Lip-Sync (`V2`)

Magic Studio delegates all character deformation and mouth visemes to [`EffectCraft` Puppet Controls](https://github.com/storytold/effectcraft/blob/main/docs/agents.md):

- `character.puppet.rig_face` installs an 11-pin facial mesh (`head_crown` & `neck_anchor` starch pins + brow, lid, cheek, mouth-corner, upper-lip, and `jaw_lower` pins).
- `character.puppet.lipsync` maps timed words from `timing-reference.json` onto mouth viseme pin keyframes.
- `character.puppet.expression` keyframes `happy`, `joy`, `surprised`, `oh`, `sad`, `focused`, `blink`, `wink_L`, `wink_R`, and `neutral`.
- `character.puppet.follow` attaches secondary follow-through (`puppet.follow`) to tails, ears, hair, or props.

## ProRes 4444 RGBA Track Publishing (`V2`, `V3`, `V4`)

`character.publish`, `words.publish`, and `motion.publish` render straight-alpha ProRes 4444 `.mov` overlays via `effectcraft-cli` and place them onto `V2`, `V3`, and `V4` in `FilmCraft` (`filmcraft-cli`) over `V1` storyboard scene plates and `A1` song audio.
