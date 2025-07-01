(progn
  (set-cfg! "cursor-colors" (list "black" "white"))
  (set-cfg! "menu-colors" (list "white" "black"))
  (set-cfg! "selection-colors" (list "red" "none"))

  (set-cfg! "key-bindings" (list
                            (list "move-down" (list (list "" "j")))
                            (list "move-up" (list (list "" "k")))
                            (list "move-left" (list (list "" "h")))
                            (list "move-right" (list (list "" "l")))
                            (list "move-top" (list (list "" "g")
                                                   (list "" "g")))
                            (list "move-bottom" (list (list "S" "G")))
                            (list "move-selection" (list (list "" "r")))
                            (list "select" (list (list "" "enter")))
                            (list "skip-song" (list (list "" "s")))
                            (list "quit" (list (list "" "q")))))

  (let ((home-dir (try-catch (lambda () (env "HOMEPATH"))
                             (lambda () (env "HOME"))))
        (music-dir (path (concat home-dir (path-separator) "Music")))
        (playlist-paths (seq-filter path-is-dir (path-children music-dir)))
        (non-empty-playlist-paths (seq-filter (lambda (path) (not (eq (nil) (path-children path))))
                                              playlist-paths))
        (playlists (seq-map
                    (lambda (playlist)
                      (list
                       (path-name playlist)
                       (seq-map
                        (lambda (song)
                          (list (path-name song) song))
                        (seq-filter
                         path-is-file
                         (path-children playlist)))))
                    non-empty-playlist-paths)))
    (set-cfg! "playlists" playlists)))
