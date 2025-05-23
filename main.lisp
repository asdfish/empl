(progn
  (set-cfg! "cursor-colors" (list "black" "white"))
  (set-cfg! "menu-colors" (list "white" "black"))
  (set-cfg! "selection-colors" (list "red" "none"))
  (let ((music-dir (path (concat (env "HOME") (path-separator) "files" (path-separator) "music")))
	(playlist-paths (seq-filter path-is-dir (path-children music-dir)))
	(playlists (seq-map
		    (lambda (playlist)
		      (list
		       (path-name playlist)
		       (seq-filter path-is-file (path-children playlist))))
		    playlist-paths)))
    (set-cfg! "playlists" playlists))
