import { Collapse } from 'bootstrap';
import { HTTP, JQ } from './http.js';

interface FileTypeFileData {
    size: number
}
interface FileTypeFile {
    File: FileTypeFileData
}

interface FileTypeDirectoryData {
    children: number
}

interface FileTypeDirectory {
    Directory: FileTypeDirectoryData
}

interface File {
    filename: string,
    file_type: FileTypeFile,
    created: Date,
    last_access: Date,
    last_modified: Date
}

interface Directory {
    filename: string,
    file_type: FileTypeDirectory,
    created: Date,
    last_access: Date,
    last_modified: Date
}

type FSItem = File | Directory;

function Finish() {}

let waiter = null;

function UIStartAwait() {
    if (!waiter) {
        waiter = document.createElement("div");
        let spinner = document.createElement("div");

        spinner.classList.add("spinner-border");
        spinner.setAttribute("style", "width: 3rem; height: 3rem; margin: auto;");

        waiter.append(spinner);
        waiter.setAttribute("style", "position: fixed; top: 75px; left: 280px; bottom: 0px; right: 0px");
        waiter.setAttribute("name", "UIAwaitForOperation");
        waiter.classList.add("d-flex", "flex-column", "justify-content-center", "text-center", "bg-light", "bg-opacity-25");
    }

    document.querySelector("body").append(waiter);
}

function UIStopAwait() {
    if (waiter) waiter.remove();
}

async function sleep(timeout: number): Promise<void> {
    return new Promise<void>((resolve, reject) => {
        setTimeout(resolve, timeout);
    });
}

class CachedFile {
    filename: string;
    created: Date;
    last_access: Date;
    last_modified: Date;
    size: number;

    constructor(original: File) {
        this.filename = original.filename;
        this.created = original.created;
        this.last_access = original.last_access;
        this.last_modified = original.last_modified;
        this.size = original.file_type.File.size;
    }
}

class CachedDirectory {
    filename: string;
    created: Date;
    last_access: Date;
    last_modified: Date;
    children: Cache;

    constructor(original: Directory) {
        this.filename = original.filename;
        this.created = original.created;
        this.last_access = original.last_access;
        this.last_modified = original.last_modified;
        this.children = new Cache();
    }
}

type CachedItem = CachedFile | CachedDirectory;

class Cache {
    items: CachedItem[];

    constructor() {
        this.items = [];
    }

    retrieve(path: string[]): CachedItem {
        let first = path.shift();

        for (let item of this.items) {
            if (item.filename !== first) continue;
            if (path.length === 0) return item;
            if (!("children" in item)) return undefined;
            item.children.retrieve(path);
        }
    }

    push(element: CachedItem) {
        this.items.push(element);
    }

    clear() {
        this.items = [];
    }
}

class FileExplorer {
    cache: Cache;
    element_DirectoryTree: Element;
    element_FileExplorer: Element;
    element_Breadcrumbs: Element;
    path: string;
    path_history: string[];
    path_history_selector: number;
    user: string;
    view: FSItem[];


    constructor() {
        this.element_DirectoryTree = document.querySelector("#DirectoryTree");
        this.element_FileExplorer = document.querySelector("#FileViewer");
        this.element_Breadcrumbs = document.querySelector("#Breadcrumbs");

        this.cache = new Cache();
        this.path = '/';
        this.path_history = ['/'];
        this.path_history_selector = 0;
        this.user = '.public';
        this.view = [];

        UIStartAwait();

        this.load_root()
            .then(this.on_load.bind(this));
    }

    create_directory_tree_item(directory: Directory): Element {
        let new_element_header_expand = document.createElement("i");
        let new_element_header_icon = document.createElement("img");
        let new_element_header_filename = document.createElement("span");
        let new_element_header = document.createElement("header");

        let new_element_body = document.createElement("ul");
        new_element_body.classList.add("tree-view", "collapse");

        if (directory.file_type.Directory.children > 0) {
            new_element_header_expand.classList.add("bi-plus-square-dotted");
            new_element_header_expand.addEventListener("click", this.on_directory_tree_item_collapse);

            for (let i = 0; i < directory.file_type.Directory.children; i++) {
                let new_element_body_item_header_placeholder = document.createElement("span");
                let new_element_body_item_header = document.createElement("header");
                let new_element_body_item = document.createElement("li");

                new_element_body_item_header_placeholder.classList.add("placeholder", "col-9", "m-2");

                new_element_body_item_header.classList.add("placeholder-glow");
                new_element_body_item_header.append(new_element_body_item_header_placeholder);

                new_element_body_item.classList.add("tree-item", "pt-1");
                new_element_body_item.append(new_element_body_item_header);

                new_element_body.append(new_element_body_item);
            }
        }

        new_element_header_icon.classList.add("m-1", "mb-2");
        new_element_header_icon.src = "/static/small/folder-vertical.png";
        new_element_header_icon.width = 16;
        new_element_header_icon.addEventListener("dblclick", this.on_directory_tree_item_dblclick);
        new_element_header_filename.classList.add("filename");
        new_element_header_filename.innerText = directory.filename;
        new_element_header_filename.addEventListener("dblclick", this.on_directory_tree_item_dblclick);
        new_element_header.append(new_element_header_expand);
        new_element_header.append(new_element_header_icon);
        new_element_header.append(new_element_header_filename);

        let new_element = document.createElement("li");
        new_element.classList.add("tree-item", "pt-1");
        new_element.append(new_element_header);
        new_element.append(new_element_body);

        return new_element;
    }

    create_file_viewer_item(item: FSItem): Element {
        let new_element_icon = document.createElement("img");
        let new_element_filename = document.createElement("div");
        let new_element = document.createElement("li");

        if ("Directory" in item.file_type) {
            new_element_icon.src = "/static/large/folder.png";
        } else {
            switch (item.filename.split(".").slice(-1)[0]) {
                case "png":
                case "jpg":
                case "jpeg":
                    new_element_icon.src = "/static/large/picture.png";
                    break;
                case "exe":
                    new_element_icon.src = "/static/large/executable.png";
                    break;
                case "txt":
                case "log":
                case "md":
                    new_element_icon.src = "/static/large/file-text.png";
                    break;
                case "ini":
                case "inf":
                case "toml":
                    new_element_icon.src = "/static/large/file-config.png";
                    break;
                case "pdf":
                    new_element_icon.src = "/static/large/file-pdf.png";
                    break;
                default:
                    new_element_icon.src = "/static/large/file.png";
                    break;
            }
        }

        new_element_filename.classList.add("filename");
        new_element_filename.innerText = item.filename;

        new_element.append(new_element_icon);
        new_element.append(new_element_filename);
        new_element.addEventListener("dblclick", this.on_file_viewer_item_dblclick);

        return new_element;
    }

    navigate_to(path: string) {
        this.path_history = this.path_history.slice(0, this.path_history_selector + 1);
        this.path_history_selector = this.path_history.length;
        this.path_history.push(path);

        let undo_button = <HTMLButtonElement>document.querySelector("#FileExplorerButton_Undo")
        let redo_button = <HTMLButtonElement>document.querySelector("#FileExplorerButton_Redo");
        let parent_button = <HTMLButtonElement>document.querySelector("#FileExplorerButton_Parent");
        undo_button.disabled = false;
        redo_button.disabled = true;
        parent_button.disabled = path === "/" || path === "";

        this.path = path;
        this.load_file_viewer_view()
            .then(this.on_file_viewer_path_changed.bind(this));
    }

    read_directory_tree_element_path(element: Element) {
        let path = "";

        while (element) {
            while (element.tagName !== "LI" && element !== this.element_DirectoryTree) {
                element = element.parentElement;
                if (element === undefined) break;
            }

            if (element === this.element_DirectoryTree) break;
            let element_filename = element.querySelector("header > span.filename");
            path = `/${element_filename.textContent}${path}`;

            element = element.parentElement;
        }

        return path;
    }

    update_navigation_buttons() {
        let undo_button = <HTMLButtonElement>document.querySelector("#FileExplorerButton_Undo")
        let redo_button = <HTMLButtonElement>document.querySelector("#FileExplorerButton_Redo");
        let parent_button = <HTMLButtonElement>document.querySelector("#FileExplorerButton_Parent");
        undo_button.disabled = this.path_history_selector === 0;
        redo_button.disabled = this.path_history_selector === this.path_history.length - 1;
        parent_button.disabled = this.path === "/" || this.path === "";
    }

    async load_root() {
        let response = await HTTP.GET(`/v1/directory/${this.user}/`)
            .send<Array<FSItem>>();

        this.element_DirectoryTree.innerHTML = "";
        this.element_FileExplorer.innerHTML = "";

        for (let directory of response) {
            if (!("Directory" in directory.file_type)) continue;
            this.cache.push(new CachedDirectory(<Directory>directory));

            let new_directory_tree_element = this.create_directory_tree_item(<Directory>directory);
            let new_file_viewer_element = this.create_file_viewer_item(directory);

            this.element_DirectoryTree.append(new_directory_tree_element);
            this.element_FileExplorer.append(new_file_viewer_element);
        }

        for (let file of response) {
            if (!("File" in file.file_type)) continue;
            this.cache.push(new CachedFile(<File>file));

            let new_file_viewer_element = this.create_file_viewer_item(file);

            this.element_FileExplorer.append(new_file_viewer_element);
        }

        this.view = response;
    }

    async load_directory_tree_children(element: Element) {
        let path = this.read_directory_tree_element_path(element);

        let response = await HTTP.GET(`/v1/directory/${this.user}${path}`)
            .send<Array<FSItem>>();

        while (element.tagName !== "LI" && element !== this.element_DirectoryTree) {
            element = element.parentElement;
        }

        let element_body = element.querySelector("ul.tree-view");

        element_body.innerHTML = "";

        for (let directory of response) {
            if (!("Directory" in directory.file_type)) continue;

            let new_element = this.create_directory_tree_item(<Directory>directory);

            element_body.append(new_element);
        }

        let element_collapse = element.querySelector("header > i");
        element_collapse.classList.remove("spinner-border", "spinner-border-sm");
        element_collapse.classList.add("bi-dash-square");
    }

    async load_file_viewer_view() {
        let response = await HTTP.GET(`/v1/directory/${this.user}${this.path}`)
            .send<FSItem[]>();

        this.element_FileExplorer.innerHTML = "";

        for (let file_item of response) {
            if (!("Directory" in file_item.file_type)) continue;

            let new_element = this.create_file_viewer_item(file_item);

            this.element_FileExplorer.append(new_element);
        }

        for (let file_item of response) {
            if (!("File" in file_item.file_type)) continue;

            let new_element = this.create_file_viewer_item(file_item);

            this.element_FileExplorer.append(new_element);
        }

        this.view = response;
    }

    on_breadcrumbs_item_click(this: Element) {
        let sender = this;
        let file_explorer = <FileExplorer>document["file_explorer"];

        while (sender.tagName !== "LI" && sender !== file_explorer.element_Breadcrumbs) {
            sender = sender.parentElement;
        }

        let path = "";
        while (sender && !sender.querySelector("a > img")) {
            path = `/${sender.querySelector("a").textContent}${path}`;
            sender = sender.previousElementSibling;
        }

        file_explorer.navigate_to(path);
    }

    on_directory_tree_item_collapse(this: Element) {
        let sender = this;
        let file_explorer = <FileExplorer>document["file_explorer"];

        if (sender.classList.contains("bi-dash-square")) {
            let element = sender.parentElement.nextElementSibling;
            new Collapse(element);

            sender.classList.remove("bi-dash-square");
            sender.classList.add("bi-plus-square");

            let adjacent_image = <HTMLImageElement>sender.nextElementSibling;
            adjacent_image.src = "/static/small/folder-vertical.png";
        } else if (sender.classList.contains("bi-plus-square")) {
            let element = sender.parentElement.nextElementSibling;
            new Collapse(element);

            sender.classList.remove("bi-plus-square");
            sender.classList.add("bi-dash-square");

            let adjacent_image = <HTMLImageElement>sender.nextElementSibling;
            adjacent_image.src = "/static/small/folder-vertical-open.png";
        } else if (sender.classList.contains("bi-plus-square-dotted")) {
            file_explorer.load_directory_tree_children(sender);

            let element = sender.parentElement.nextElementSibling;
            new Collapse(element);

            sender.classList.remove("bi-plus-square-dotted");
            sender.classList.add("spinner-border", "spinner-border-sm");

            let adjacent_image = <HTMLImageElement>sender.nextElementSibling;
            adjacent_image.src = "/static/small/folder-vertical-open.png";
        }

        return false;
    }

    on_directory_tree_item_dblclick(this: Element) {
        let sender = this;
        let file_explorer = <FileExplorer>document["file_explorer"];

        let path = file_explorer.read_directory_tree_element_path(sender);
        file_explorer.navigate_to(path);
    }

    on_file_viewer_path_changed() {
        let user_element_icon = document.createElement("img");
        let user_element_link = document.createElement("a");
        let user_element = document.createElement("li");

        //user_element_icon.classList.add("m-0", "mb-1");

        if (this.user === '.public') {
            user_element_icon.src = "/static/small/world.png";
            //user_element_icon.classList.add("bi-globe");
            user_element_link.append(user_element_icon);
            user_element_link.append(` Public`);
        } else {
            user_element_icon.src = "/static/small/user.png";
            user_element_link.append(user_element_icon);
            user_element_link.append(` ${this.user}`);
        }
        user_element_link.href = "#";
        user_element_link.classList.add("icon-link");
        user_element_link.addEventListener("click", this.on_breadcrumbs_item_click);

        user_element.append(user_element_link);
        user_element.classList.add("breadcrumb-item");

        this.element_Breadcrumbs.innerHTML = "";
        this.element_Breadcrumbs.append(user_element);

        if (this.path == '/') return;

        let path = this.path.split("/");
        // Correction for first element being /
        path.shift();

        for (let path_item of path) {
            let new_element_link = document.createElement("a");
            let new_element = document.createElement("li");

            new_element_link.innerText = path_item;
            new_element_link.href = "#";
            new_element_link.classList.add("icon-link");
            new_element_link.addEventListener("click", this.on_breadcrumbs_item_click);

            new_element.append(new_element_link);
            new_element.classList.add("breadcrumb-item");

            this.element_Breadcrumbs.append(new_element);
        }
    }

    on_file_viewer_item_dblclick(this: Element) {
        let sender = this;
        let file_explorer = <FileExplorer>document["file_explorer"];

        while (sender.tagName !== "LI" && sender !== file_explorer.element_FileExplorer) {
            sender = sender.parentElement;
        }

        let filename = sender.querySelector("div.filename")
            .textContent;

        for (let file of file_explorer.view) {
            if (file.filename !== filename) continue;

            if (!("Directory" in file.file_type)) {
                // TODO: handle file recognition, etc.
                console.log("Unable to parse file.");
                break;
            }

            let path = file_explorer.path;
            if (path === '/') path = '';

            file_explorer.navigate_to(`${path}/${filename}`);

            break;
        }
    }

    on_load() {
        document.querySelector('#FileExplorerButton_Undo').addEventListener("click", this.on_undo_button_click);
        document.querySelector('#FileExplorerButton_Redo').addEventListener("click", this.on_redo_button_click);
        document.querySelector('#FileExplorerButton_Parent').addEventListener("click", this.on_parent_button_click);

        this.update_navigation_buttons();

        UIStopAwait();
    }

    on_parent_button_click(this: Element) {
        let file_explorer = <FileExplorer>document["file_explorer"];

        let exploded_path = file_explorer.path.split('/');
        exploded_path.pop();
        let path = exploded_path.join('/');

        file_explorer.navigate_to(path);
    }

    on_undo_button_click(this: Element) {
        let file_explorer = <FileExplorer>document["file_explorer"];

        if (file_explorer.path_history_selector > 0) --file_explorer.path_history_selector;
        file_explorer.path = file_explorer.path_history[file_explorer.path_history_selector];

        file_explorer.update_navigation_buttons();
        file_explorer.load_file_viewer_view()
            .then(file_explorer.on_file_viewer_path_changed.bind(file_explorer));
    }

    on_redo_button_click(this: Element) {
        let file_explorer = <FileExplorer>document["file_explorer"];

        if (file_explorer.path_history_selector < file_explorer.path_history.length) ++file_explorer.path_history_selector;
        file_explorer.path = file_explorer.path_history[file_explorer.path_history_selector];

        file_explorer.update_navigation_buttons();
        file_explorer.load_file_viewer_view()
            .then(file_explorer.on_file_viewer_path_changed.bind(file_explorer));
    }
}

JQ(() => {
    document["file_explorer"] = new FileExplorer();
} );