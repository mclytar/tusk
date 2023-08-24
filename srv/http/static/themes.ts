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
    name: string;

    constructor(name: string) {
        this.name = name;
    }

    apply_to(element: Element) {
        let image_large = document.createElement("img");
        if (ICON_STYLES[this.name]) {
            image_large.src = `/static/ui/file-types/large/${ICON_STYLES[this.name]}.png`;
        } else {
            image_large.src = `/static/ui/file-types/large/file.png`;
        }
        image_large.classList.add(`icon-large`);
        element.append(image_large);

        let image_medium = document.createElement("img");
        if (ICON_STYLES[this.name]) {
            image_medium.src = `/static/ui/file-types/large/${ICON_STYLES[this.name]}.png`;
        } else {
            image_medium.src = `/static/ui/file-types/large/file.png`;
        }
        image_medium.classList.add(`icon-medium`);
        element.append(image_medium);

        let image_small = document.createElement("img");
        if (ICON_STYLES[this.name]) {
            image_small.src = `/static/ui/file-types/small/${ICON_STYLES[this.name]}.png`;
        } else {
            image_small.src = `/static/ui/file-types/small/file.png`;
        }
        image_small.classList.add(`icon-small`);
        element.append(image_small);
    }
}

export enum IconSize {
    Large = "large",
    Medium = "medium",
    Small = "small"
}

export class IconSet {
    path_large: string;
    path_medium: string;
    path_small: string;

    apply_to(element: Element) {
        for (let size of ["large", "medium", "small"]) {
            let image = document.createElement("img");
            image.src = this[`path_${size}`];
            image.classList.add(`icon-${size}`);
            element.append(image);
        }
    }
}

export class Theme {
    name: string;
    directories: {
        "large": string,
        "medium": string,
        "small": string
    };
    filename_map: {
        "default": string,
        "map": Map<string, string>
    };
    image_type: string;

    constructor(name: string) {
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

    static build(name: string): Theme {
        return new Theme(name);
    }

    static current(): Theme {
        return __CURRENT_THEME;
    }

    large(dir: string): Theme {
        this.directories["large"] = dir;
        return this;
    }

    medium(dir: string): Theme {
        this.directories["medium"] = dir;
        return this;
    }

    small(dir: string): Theme {
        this.directories["small"] = dir;
        return this;
    }

    icon_map(map: any): Theme {
        this.filename_map["map"] = map;
        return this;
    }

    extension(ext: string): Theme {
        this.image_type = ext;
        return this;
    }

    icon_filename(name: string): string {
        let filename = this.filename_map["map"][name];
        if (!filename) filename = this.filename_map["default"];
        return filename;
    }

    icon_set(name: string): IconSet {
        let set = new IconSet();
        for (let size of ["large", "medium", "small"]) {
            set[`path_${size}`] = `/static/theme/${this.name}/${this.directories[size]}/${name}.${this.image_type}`;
        }
        return set;
    }

    icon_set_by_extension(extension: string): IconSet {
        let set = new IconSet();
        for (let size of ["large", "medium", "small"]) {
            set[`path_${size}`] = `/static/theme/${this.name}/${this.directories[size]}/${this.icon_filename(extension)}.${this.image_type}`;
        }
        return set;
    }

    src(size: IconSize, name: string): string {
        return `/static/theme/${this.name}/${this.directories[size]}/${name}.${this.image_type}`;
    }

    src_by_extension(size: IconSize, extension: string): string {
        return `/static/theme/${this.name}/${this.directories[size]}/${this.icon_filename(extension)}.${this.image_type}`;
    }
}

export enum AvailableThemes {
    Breeze = "breeze",
    FlatPro = "flat-pro",
    OfficePro = "office-pro",
    Windows = "win"
}

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

let __CURRENT_THEME: Theme = __THEME_LIST["breeze"];

window["set_theme"] = function(name: string, extension: string = "png") {
    document.querySelectorAll(".file-explorer img").forEach((img: HTMLImageElement) => {
        let icon_size = img.src.split("/").slice(-2)[0];
        let icon_name = img.src.split("/").slice(-1)[0]
            .split(".")[0];
        img.src = `/static/theme/${name}/${icon_size}/${icon_name}.${extension}`;
    });
}