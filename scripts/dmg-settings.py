# dmgbuild layout for the Rove disk image: the app on the left, Applications
# on the right, over dmgbuild's built-in arrow background.
import os.path

app = defines["app"]
files = [app]
symlinks = {"Applications": "/Applications"}
icon = defines.get("icon")
if icon and not os.path.exists(icon):
    icon = None
background = "builtin-arrow"
icon_locations = {os.path.basename(app): (140, 120), "Applications": (500, 120)}
window_rect = ((200, 160), (640, 320))
icon_size = 128
text_size = 13
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
include_icon_view_settings = True
format = "UDZO"
