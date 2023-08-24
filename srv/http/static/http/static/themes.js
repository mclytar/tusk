let ICON_STYLES = {
    "avi": "video",
    "exe": "exe",
    "ini": "conf",
    "jpg": "png",
    "jpeg": "png",
    "md": "txt",
    "mpeg": "video",
    "mp4": "video",
    "pdf": "pdf",
    "png": "png",
    "sql": "sql",
    "toml": "conf",
    "txt": "txt",
    "zip": "zip"
};
export class Icon {
    name;
    constructor(name) {
        this.name = name;
    }
    apply_to(element) {
        let image_large = document.createElement("img");
        if (ICON_STYLES[this.name]) {
            image_large.src = `/static/ui/file-types/large/${ICON_STYLES[this.name]}.png`;
        }
        else {
            image_large.src = `/static/ui/file-types/large/file.png`;
        }
        image_large.classList.add(`icon-large`);
        element.append(image_large);
        let image_medium = document.createElement("img");
        if (ICON_STYLES[this.name]) {
            image_medium.src = `/static/ui/file-types/large/${ICON_STYLES[this.name]}.png`;
        }
        else {
            image_medium.src = `/static/ui/file-types/large/file.png`;
        }
        image_medium.classList.add(`icon-medium`);
        element.append(image_medium);
        let image_small = document.createElement("img");
        if (ICON_STYLES[this.name]) {
            image_small.src = `/static/ui/file-types/small/${ICON_STYLES[this.name]}.png`;
        }
        else {
            image_small.src = `/static/ui/file-types/small/file.png`;
        }
        image_small.classList.add(`icon-small`);
        element.append(image_small);
    }
}
export var IconSize;
(function (IconSize) {
    IconSize["Large"] = "large";
    IconSize["Medium"] = "medium";
    IconSize["Small"] = "small";
})(IconSize || (IconSize = {}));
export class IconSet {
    path_large;
    path_medium;
    path_small;
    apply_to(element) {
        for (let size of ["large", "medium", "small"]) {
            let image = document.createElement("img");
            image.src = this[`path_${size}`];
            image.classList.add(`icon-${size}`);
            element.append(image);
        }
    }
}
export class Theme {
    name;
    directories;
    filename_map;
    image_type;
    constructor(name) {
        this.name = name;
        this.directories = {
            "large": "large",
            "medium": "medium",
            "small": "small"
        };
        this.filename_map = {
            "default": "file",
            "map": new Map()
        };
        this.image_type = "png";
    }
    static build(name) {
        return new Theme(name);
    }
    static current() {
        return __CURRENT_THEME;
    }
    large(dir) {
        this.directories["large"] = dir;
        return this;
    }
    medium(dir) {
        this.directories["medium"] = dir;
        return this;
    }
    small(dir) {
        this.directories["small"] = dir;
        return this;
    }
    icon_map(map) {
        this.filename_map["map"] = map;
        return this;
    }
    extension(ext) {
        this.image_type = ext;
        return this;
    }
    icon_filename(name) {
        let filename = this.filename_map["map"][name];
        if (!filename)
            filename = this.filename_map["default"];
        return filename;
    }
    icon_set(name) {
        let set = new IconSet();
        for (let size of ["large", "medium", "small"]) {
            set[`path_${size}`] = `/static/theme/${this.name}/${this.directories[size]}/${name}.${this.image_type}`;
        }
        return set;
    }
    icon_set_by_extension(extension) {
        let set = new IconSet();
        for (let size of ["large", "medium", "small"]) {
            set[`path_${size}`] = `/static/theme/${this.name}/${this.directories[size]}/${this.icon_filename(extension)}.${this.image_type}`;
        }
        return set;
    }
    src(size, name) {
        return `/static/theme/${this.name}/${this.directories[size]}/${name}.${this.image_type}`;
    }
    src_by_extension(size, extension) {
        return `/static/theme/${this.name}/${this.directories[size]}/${this.icon_filename(extension)}.${this.image_type}`;
    }
}
export var AvailableThemes;
(function (AvailableThemes) {
    AvailableThemes["Breeze"] = "breeze";
    AvailableThemes["FlatPro"] = "flat-pro";
    AvailableThemes["OfficePro"] = "office-pro";
    AvailableThemes["Windows"] = "win";
})(AvailableThemes || (AvailableThemes = {}));
let __THEME_LIST = {};
__THEME_LIST[AvailableThemes.Breeze] = Theme.build("breeze").extension("svg")
    .icon_map({
    "7z": "file-zip",
    "exe": "executable",
    "inf": "file-config",
    "ini": "file-config",
    "jpg": "picture",
    "jpeg": "picture",
    "md": "file-text",
    "pdf": "file-pdf",
    "png": "picture",
    "ppt": "file-presentation",
    "pptx": "file-presentation",
    "rar": "file-zip",
    "toml": "file-config",
    "txt": "file-text",
    "zip": "file-zip",
});
__THEME_LIST[AvailableThemes.FlatPro] = Theme.build("flat-pro").medium("large");
__THEME_LIST[AvailableThemes.OfficePro] = Theme.build("office-pro").medium("large");
__THEME_LIST[AvailableThemes.Windows] = new Theme("win");
let __CURRENT_THEME = __THEME_LIST["breeze"];
window["set_theme"] = function (name, extension = "png") {
    document.querySelectorAll(".file-explorer img").forEach((img) => {
        let icon_size = img.src.split("/").slice(-2)[0];
        let icon_name = img.src.split("/").slice(-1)[0]
            .split(".")[0];
        img.src = `/static/theme/${name}/${icon_size}/${icon_name}.${extension}`;
    });
};
//# sourceMappingURL=themes.js.map