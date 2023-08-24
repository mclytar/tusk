let ICON_STYLES = new Map;
ICON_STYLES.set("avi", "video");
ICON_STYLES.set("exe", "exe");
ICON_STYLES.set("ini", "conf");
ICON_STYLES.set("jpg", "png");
ICON_STYLES.set("jpeg", "png");
ICON_STYLES.set("md", "txt");
ICON_STYLES.set("mpeg", "video");
ICON_STYLES.set("mp4", "video");
ICON_STYLES.set("pdf", "pdf");
ICON_STYLES.set("png", "png");
ICON_STYLES.set("sql", "sql");
ICON_STYLES.set("toml", "conf");
ICON_STYLES.set("txt", "txt");
ICON_STYLES.set("zip", "zip");
export class Icon {
    name;
    constructor(name) {
        this.name = name;
    }
    apply_to(element) {
        let image_large = document.createElement("img");
        if (ICON_STYLES.get(this.name)) {
            image_large.src = `/static/ui/file-types/large/${ICON_STYLES.get(this.name)}.png`;
        }
        else {
            image_large.src = `/static/ui/file-types/large/file.png`;
        }
        image_large.classList.add(`icon-large`);
        element.append(image_large);
        let image_medium = document.createElement("img");
        if (ICON_STYLES.get(this.name)) {
            image_medium.src = `/static/ui/file-types/large/${ICON_STYLES.get(this.name)}.png`;
        }
        else {
            image_medium.src = `/static/ui/file-types/large/file.png`;
        }
        image_medium.classList.add(`icon-medium`);
        element.append(image_medium);
        let image_small = document.createElement("img");
        if (ICON_STYLES.get(this.name)) {
            image_small.src = `/static/ui/file-types/small/${ICON_STYLES.get(this.name)}.png`;
        }
        else {
            image_small.src = `/static/ui/file-types/small/file.png`;
        }
        image_small.classList.add(`icon-small`);
        element.append(image_small);
    }
}
//# sourceMappingURL=icons.js.map