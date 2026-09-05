# Movement and sound

The skeleton uses the main client's left/right step actions: walk 110/111,
run 120/121 and jump 130. Each accepted tile step alternates feet. Motion
selection and Action.dat timing use the same paired-action fallback as the
main client. Movement progress drives each step or jump pose.

ActionSound.ini selects sounds using the body, weapon family and action.
The equipped weapon tuple takes precedence over the authored 999 tuple.
Missing entries stay silent. Audio is decoded once through Rodio, the same
backend as the main client. Local sounds are centered, at the default 80%
effects volume. Missing files or an unavailable device do not stop the game.

The source was taken from the main client's movement, appearance, action-sound
catalog and native audio code, with networking, other actors, music and effect
systems left out. No sound files are included in this repository.
