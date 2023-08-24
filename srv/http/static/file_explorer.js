/// <reference path="./_core.ts" />
/// <reference path="./http.ts" />
/// <reference path="./themes.ts" />
import { Collapse, Modal } from 'bootstrap';
import { HTTP, HTTPStatusCode } from "./http.js";
import { Icon } from "./icons.js";
/**
 * A `user/path` combination.
 *
 * It is used to navigate to directories and to store information for the "undo"/"redo" buttons in the file explorer.
 */
class FullPath {
    _user;
    _path;
    /**
     * Creates a new full path given the user and the relative path.
     *
     * @param user User of the path.
     * @param path Relative path.
     */
    constructor(user, path) {
        this._user = user;
        this._path = path;
        if (!this._path.startsWith("/"))
            this._path = this._path + "/";
        while (this._path.endsWith("/"))
            this._path = this._path.slice(0, -1);
    }
    /**
     * Gets the user of the history item.
     */
    get user() {
        return this._user;
    }
    /**
     * Sets the user of the history item.
     */
    set user(value) {
        this._user = value;
    }
    /**
     * Gets the path of the history item.
     */
    get path() {
        return this._path;
    }
    /**
     * Sets the path of the history item.
     */
    set path(value) {
        this._path = value;
        if (!this._path.startsWith("/"))
            this._path = this._path + "/";
        while (this._path.endsWith("/"))
            this._path = this._path.slice(0, -1);
    }
    /**
     * Returns the full string of the path, including the user.
     *
     * If **user** is `test` and **path** is `/some/path`, then the returned string will be
     * `test/some/path`.
     */
    get() {
        let path = this._path;
        if (!path.startsWith("/"))
            path = "/" + path;
        return `${this.user}${path}`;
    }
    /**
     * Implementation of interface {@link PartialEq}.
     * @param other Other object of type **FullPath** to compare with.
     */
    partial_eq(other) {
        if (this._user !== other._user)
            return false;
        return this._path === other._path;
    }
}
/**
 * History of the visited locations.
 *
 * It is used to store information for the "undo"/"redo" buttons.
 */
class History {
    /**
     * Items stored in the history.
     */
    list;
    /**
     * Position in the history.
     */
    selector;
    /**
     * Constructs a new history.
     *
     * @param first First item in the history.
     */
    constructor(first) {
        this.list = [first];
        this.selector = 0;
    }
    /**
     * Visit a new item, discarding all the future items.
     *
     * @param item Item to visit.
     */
    visit(item) {
        if (this.list[this.selector].partial_eq(item))
            return;
        this.list = this.list.slice(0, this.selector + 1);
        this.selector = this.list.length;
        this.list.push(item);
    }
    /**
     * Retrieves the current history item (_i.e.,_ the **present**).
     */
    current() {
        return this.list[this.selector];
    }
    /**
     * Goes back by one item, if possible, and returns the item.
     */
    go_back() {
        if (!this.is_first())
            --this.selector;
        return this.list[this.selector];
    }
    /**
     * Goes forth by one item, if possible, and returns the item.
     */
    go_forth() {
        if (!this.is_last())
            ++this.selector;
        return this.list[this.selector];
    }
    /**
     * Returns `true` if the current item is the first one (_i.e._, it is not possible to go back), and `false`
     * otherwise.
     */
    is_first() {
        return this.selector === 0;
    }
    /**
     * Returns `true` if the current item is the last one (_i.e._, it is not possible to go forth), and `false`
     * otherwise.
     */
    is_last() {
        return this.selector === this.list.length - 1;
    }
}
/**
 * Represents an item in the directory tree.
 *
 * Provides the methods and handles the events relative to the designated directory tree item.
 */
class DirectoryTreeItem {
    element;
    file_explorer;
    /**
     * Constructs a new directory tree item.
     *
     * @param file_explorer {@link HTMLFileExplorer} object to which this item belongs.
     * @param directory Descriptor of the directory as received by the API endpoint.
     */
    constructor(file_explorer, directory) {
        let header_expand_button = document.createElement("i");
        let header_icon = document.createElement("img");
        let header_filename = document.createElement("span");
        let header = document.createElement("header");
        let body = document.createElement("ul");
        let element = document.createElement("li");
        if (directory.children > 0) {
            header_expand_button.classList.add("bi-plus-square-dotted");
            header_expand_button.addEventListener("click", this.on_collapse_button_click.bind(this));
            for (let i = 0; i < directory.children; i++) {
                let header_filename = document.createElement("span");
                let header = document.createElement("header");
                let element = document.createElement("li");
                header_filename.classList.add("placeholder", "col-9", "m-2");
                header.classList.add("placeholder-glow");
                element.classList.add("tree-item", "pt-1");
                header.append(header_filename);
                element.append(header);
                body.append(element);
            }
        }
        header_icon.classList.add("m-1", "mb-2");
        header_icon.src = "/static/ui/light/small/folder-vertical.png";
        header_icon.sizes = "(max-width: 16px) 16px, (max-width: 32px) 32px, 64px";
        header_icon.width = 16;
        header_icon.addEventListener("dblclick", this.on_dblclick.bind(this));
        header_filename.classList.add("filename");
        header_filename.innerText = directory.filename;
        header_filename.addEventListener("dblclick", this.on_dblclick.bind(this));
        body.classList.add("tree-view", "collapse");
        element.classList.add("tree-item", "pt-1");
        header.append(header_expand_button);
        header.append(header_icon);
        header.append(header_filename);
        element.append(header);
        element.append(body);
        this.element = element;
        this.file_explorer = file_explorer;
    }
    /**
     * Loads all the child elements of the current directory tree item.
     */
    async load_children() {
        let path = this.path();
        let user = this.file_explorer.user();
        let response = await HTTP.GET(`/v1/directory/${user}${path}`)
            .Accept("application/json")
            .send();
        if (response.status() !== HTTPStatusCode.OK)
            throw new Error("Unable to retrieve directory.");
        let container = this.element.querySelector(":scope > ul");
        if (!container)
            throw new Error("No directory tree container found.");
        container.innerHTML = "";
        for (let directory of response.body()) {
            if (directory.kind !== "directory")
                continue;
            let directory_tree_item = new DirectoryTreeItem(this.file_explorer, directory);
            container.append(directory_tree_item.element);
        }
    }
    /**
     * Retrieves the path to which this directory tree item refers.
     */
    path() {
        let current_element = this.element;
        let path = "";
        while (current_element) {
            let span_filename = current_element.querySelector(":scope > header > span.filename");
            if (!span_filename)
                throw new Error("Cannot find filename");
            path = "/" + span_filename.textContent + path;
            let parent = current_element.parentElement;
            if (!parent)
                throw new Error("Cannot find parent element");
            current_element = parent.closest("li");
        }
        if (path === "")
            path = "/";
        return path;
    }
    /**
     * Handles the click event of the "collapse" button.
     *
     * When the "collapse" button is pressed, the list of subdirectories of this item
     * expands or collapses depending on whether it was previously collapsed or expanded.
     * Additionally, if the content has never been loaded before (i.e., the "collapse" button is dotted),
     * this event loads the content of the list of subdirectories.
     *
     * @param e MouseEvent object relative to the fired event.
     */
    on_collapse_button_click(e) {
        let sender = e.currentTarget instanceof HTMLElement ? e.currentTarget : null;
        if (!sender)
            throw new Error("Event has no sender");
        let element = sender.closest("li");
        if (!element)
            throw new Error("Element has no parent");
        let children_list = element.querySelector("ul");
        if (!children_list)
            throw new Error("Element has no attached children list");
        new Collapse(children_list);
        if (sender.classList.contains("bi-dash-square")) {
            sender.classList.remove("bi-dash-square");
            sender.classList.add("bi-plus-square");
        }
        else if (sender.classList.contains("bi-plus-square")) {
            sender.classList.remove("bi-plus-square");
            sender.classList.add("bi-dash-square");
        }
        else if (sender.classList.contains("bi-plus-square-dotted")) {
            sender.classList.remove("bi-plus-square-dotted");
            sender.classList.add("spinner-border", "spinner-border-sm");
            let on_load = () => {
                let local_sender = sender;
                local_sender.classList.remove("spinner-border", "spinner-border-sm");
                local_sender.classList.add("bi-dash-square");
                local_sender.dispatchEvent(new Event("content_load"));
            };
            this.load_children()
                .then(on_load);
        }
    }
    on_dblclick() {
        let full_path = this.file_explorer.path.current();
        let path = this.path();
        this.file_explorer.navigate_to(new FullPath(full_path.user, path));
    }
}
/**
 * Represents an item in the file viewer.
 *
 * Provides the methods and handles the events relative to the designated file viewer item.
 */
class FileViewerItem {
    element;
    file_explorer;
    /**
     * Constructs a new file viewer item.
     *
     * @param file_explorer {@link HTMLFileExplorer} object to which this item belongs.
     * @param item Descriptor of the file or directory as received by the API endpoint.
     */
    constructor(file_explorer, item) {
        let content_filename = document.createElement("div");
        let detail_size = document.createElement("div");
        let detail_created = document.createElement("div");
        let detail_modified = document.createElement("div");
        let element = document.createElement("li");
        if (item.kind === "directory") {
            let image_large = document.createElement("img");
            image_large.src = "/static/ui/light/large/folder.png";
            image_large.classList.add("icon-large");
            element.append(image_large);
            let image_medium = document.createElement("img");
            image_medium.src = "/static/ui/light/large/folder.png";
            image_medium.classList.add("icon-medium");
            element.append(image_medium);
            let image_small = document.createElement("img");
            image_small.src = "/static/ui/light/small/folder.png";
            image_small.classList.add("icon-small");
            element.append(image_small);
        }
        else {
            let extension = item.filename.split(".").slice(-1)[0];
            new Icon(extension)
                .apply_to(element);
        }
        content_filename.classList.add("filename");
        content_filename.innerText = item.filename;
        if (item.kind === "file") {
            let size = item.size;
            let supported_measures = ["B", "kiB", "MiB", "GiB", "TiB"];
            let measure = supported_measures.shift();
            while (size > 1024) {
                measure = supported_measures.shift();
                if (!measure) {
                    measure = "TiB";
                    break;
                }
                size = size / 1024;
            }
            size = Math.floor(size * 100) / 100;
            detail_size.innerText = `${size} ${measure}`;
        }
        let creation_date = new Date(item.created * 1000);
        detail_created.innerText = creation_date.toLocaleString();
        detail_created.classList.add("created");
        let modification_date = new Date(item.last_modified * 1000);
        detail_modified.innerText = modification_date.toLocaleString();
        detail_modified.classList.add("modified");
        detail_size.classList.add("size");
        element.append(content_filename);
        element.append(detail_size);
        element.append(detail_created);
        element.append(detail_modified);
        element.addEventListener("click", this.on_click.bind(this));
        element.addEventListener("dblclick", this.on_dblclick.bind(this));
        this.element = element;
        this.file_explorer = file_explorer;
    }
    on_click(e) {
        let file_viewer = this.file_explorer.root.querySelector(`ul.sf-component-file-viewer`);
        if (!file_viewer)
            throw new Error("No file viewer detected.");
        if (!e.ctrlKey) {
            for (let item of file_viewer.children) {
                item.classList.remove("selected");
            }
        }
        let sender = e.currentTarget;
        sender?.classList.add("selected");
        this.file_explorer.on_selection_change();
    }
    /**
     * Handles the dblclick event of a file viewer item.
     *
     * A file viewer item is either a file or a directory.
     * Upon double-clicking a file, if the operation is supported, the browser will show the contents
     * of the file; otherwise, it displays a popup saying that the file format is not supported,
     * giving the possibility to download the file.
     * Upon double-clicking a folder, the file explorer object updates to display the contents of the folder.
     *
     * @param e MouseEvent object relative to the fired event.
     */
    on_dblclick(e) {
        let sender = e.currentTarget;
        if (!sender)
            throw new Error("Dblclick event has no sender.");
        let div_filename = sender.querySelector("div.filename");
        if (!div_filename)
            throw new Error("Sender has no attached filename.");
        let filename = div_filename.textContent;
        let full_path = this.file_explorer.path.current();
        let path = full_path.path;
        if (!path.endsWith("/"))
            path = path + "/";
        path = path + filename;
        let size = sender.querySelector(".size");
        if (size && size.textContent !== "") {
            this.file_explorer.open(new FullPath(full_path.user, path));
        }
        else {
            this.file_explorer.navigate_to(new FullPath(full_path.user, path));
        }
    }
}
/**
 * Represents an item in the breadcrumb list of the file explorer.
 *
 * Provides the methods and handles the events relative to the designated breadcrumb item.
 */
class BreadcrumbItem {
    element;
    file_explorer;
    /**
     * Constructs a new breadcrumb item.
     *
     * @param file_explorer {@link HTMLFileExplorer} object to which this item belongs.
     * @param item Component of the directory path.
     */
    constructor(file_explorer, item) {
        let element_link = document.createElement("a");
        let element = document.createElement("li");
        element_link.innerText = item;
        element_link.href = "#";
        element_link.classList.add("icon-link");
        element_link.addEventListener("click", this.on_click.bind(this));
        element.append(element_link);
        element.classList.add("breadcrumb-item");
        this.element = element;
        this.file_explorer = file_explorer;
    }
    /**
     * Retrieves the path to which this directory tree item refers.
     */
    path() {
        let current_element = this.element;
        let path = "";
        while (current_element) {
            let a_filename = current_element.querySelector(":scope > a");
            if (!a_filename)
                break;
            path = "/" + a_filename.textContent + path;
            current_element = current_element.previousElementSibling;
        }
        if (path === "")
            path = "/";
        return path;
    }
    on_click() {
        let full_path = new FullPath(this.file_explorer.path.current().user, this.path());
        this.file_explorer.navigate_to(full_path);
    }
}
class TreeNode {
    static size(node) {
        let result = 0;
        for (let _ in node) {
            result += 1;
        }
        return result;
    }
}
/**
 * Represents a structure of HTML elements displaying a file explorer.
 *
 * Provides the methods and handles the events relative to the file explorer.
 */
class FileExplorer {
    root;
    path;
    /**
     * The **HTMLFileExplorer** constructor constructs a wrapper to a file explorer.
     * @param root Root element containing all the structure for the file viewer.
     * @param user Initial user of the file viewer.
     * @param path Initial path for the file viewer.
     */
    constructor(root, user = null, path = "/") {
        // Initialize the object.
        if (typeof root === "string") {
            let maybe_root = document.querySelector(root);
            if (!maybe_root)
                throw new Error("Element not found.");
            this.root = maybe_root;
        }
        else {
            this.root = root;
        }
        // @ts-ignore
        if (!user)
            user = document['username'] ? document['username'] : ".public";
        this.path = new History(new FullPath(user, path));
        // Asynchronously load the file explorer, firing the on_load event when done.
        this.__load().then(this.on_load.bind(this));
    }
    directory_tree() {
        let tree = new TreeNode();
        let explorer = (parent, tree) => {
            for (let element of parent.children) {
                let filename = element.querySelector(":scope > header > span.filename")?.textContent;
                if (!filename)
                    continue;
                tree[filename] = new TreeNode();
                let maybe_has_content = element.querySelector(" :scope > header > i.bi-dash-square");
                if (!maybe_has_content)
                    continue;
                let content = element.querySelector(":scope > ul");
                if (content && content.innerHTML !== "")
                    explorer(content, tree[filename]);
            }
        };
        explorer(this.directory_tree_element(), tree);
        return tree;
    }
    /**
     * Returns the element containing the directory tree of the file explorer.
     */
    directory_tree_element() {
        let directory_tree = this.root.querySelector("main .sf-component-directory-tree");
        if (!directory_tree)
            throw new Error("File explorer does not have directory tree.");
        return directory_tree;
    }
    /**
     * Returns the element containing the file viewer of the file explorer.
     */
    file_viewer_element() {
        let directory_tree = this.root.querySelector("main .sf-component-file-viewer");
        if (!directory_tree)
            throw new Error("File explorer does not have directory tree.");
        return directory_tree;
    }
    directory_tree_by_path(path) {
        if (!path)
            path = this.path.current().path;
        while (path.startsWith("/"))
            path = path.slice(1);
        while (path.endsWith("/"))
            path = path.slice(0, -1);
        let element = this.directory_tree_element();
        if (path === "")
            return element;
        for (let component of path?.split("/")) {
            let children = element.querySelectorAll(":scope > li");
            let found = false;
            for (let child of children) {
                let filename = child.querySelector(":scope > header > span.filename");
                if (!filename)
                    continue;
                if (filename.textContent !== component)
                    continue;
                let maybe_element = child.querySelector(":scope > ul.tree-view");
                if (!maybe_element)
                    continue;
                element = maybe_element;
                found = true;
                break;
            }
            if (!found)
                return null;
        }
        return element;
    }
    async create_folder(filename) {
        let path = this.path.current();
        let data = new FormData();
        let blob = new Blob([JSON.stringify({
                "kind": "Folder",
                "name": filename
            })], {
            type: "application/json"
        });
        data.set("metadata", blob, "");
        let response = await HTTP.POST(`/v1/directory/${path.get()}`)
            .Accept("application/json")
            .body_form(data)
            .send();
        if (response.status() !== HTTPStatusCode.CREATED)
            throw new Error("Unable to create directory.");
        await this.load_view();
    }
    async upload_file(file) {
        let path = this.path.current();
        let data = new FormData();
        let blob = new Blob([JSON.stringify({
                "kind": "File",
                "name": file.name
            })], {
            type: "application/json"
        });
        data.set("metadata", blob, "");
        data.set("payload", file);
        let response = await HTTP.POST(`/v1/directory/${path.get()}`)
            .Accept("application/json")
            .body_form(data)
            .send();
        if (response.status() !== HTTPStatusCode.CREATED)
            throw new Error("Unable to create directory.");
        await this.load_view();
    }
    async delete_file(filename) {
        let path = this.path.current().get();
        if (!path.endsWith("/"))
            path = path + "/";
        let response = await HTTP.DELETE(`/v1/directory/${path}${filename}`)
            .Accept("application/json")
            .send();
        if (response.status() !== HTTPStatusCode.OK)
            throw new Error("Unable to delete file.");
        await this.load_view();
    }
    async load_view() {
        let path = this.path.current();
        let response = await HTTP.GET(`/v1/directory/${path.get()}`)
            .Accept("application/json")
            .send();
        if (response.status() !== HTTPStatusCode.OK)
            throw new Error("Unable to retrieve directory.");
        let file_viewer = this.file_viewer_element();
        let directory_tree = this.directory_tree_by_path(path.path);
        let collapse_button = directory_tree?.parentElement?.querySelector(":scope > header > i.bi-plus-square-dotted");
        if (!collapse_button)
            directory_tree = null;
        file_viewer.innerHTML = `<header><div></div><div>Name</div><div>Size</div><div>Created</div><div>Last modified</div></header>`;
        if (directory_tree)
            directory_tree.innerHTML = "";
        for (let directory of response.body()) {
            if (directory.kind !== "directory")
                continue;
            let directory_tree_item = new DirectoryTreeItem(this, directory);
            let file_viewer_item = new FileViewerItem(this, directory);
            directory_tree?.append(directory_tree_item.element);
            file_viewer.append(file_viewer_item.element);
        }
        for (let file of response.body()) {
            if (file.kind !== "file")
                continue;
            let file_viewer_item = new FileViewerItem(this, file);
            file_viewer.append(file_viewer_item.element);
        }
        collapse_button?.classList.remove("bi-plus-square-dotted");
        collapse_button?.classList.add("bi-plus-square");
        this.on_selection_change();
    }
    open(path) {
        window.open(`/v1/directory/${path.get()}`, '_blank');
    }
    navigate_to(path) {
        this.path.visit(path);
        this.load_view()
            .then(this.on_path_changed.bind(this));
    }
    update_breadcrumb() {
        let breadcrumb = this.root.querySelector(".sf-component-breadcrumb");
        if (!breadcrumb)
            return;
        breadcrumb.innerHTML = "";
        let first = document.createElement("li");
        first.classList.add("breadcrumb-item");
        breadcrumb.append(first);
        if (this.path.current().path === "/")
            return;
        let path = this.path.current().path.split("/");
        path.shift();
        for (let component of path) {
            let item = new BreadcrumbItem(this, component);
            breadcrumb.append(item.element);
        }
    }
    user() {
        return this.path.current().user;
    }
    /**
     * Asynchronously loads the file explorer element.
     *
     * @private
     */
    async __load() {
        let response = await HTTP.GET(`/v1/directory/${this.user()}/`)
            .Accept("application/json")
            .send();
        if (response.status() !== HTTPStatusCode.OK)
            throw new Error("Unable to retrieve directory.");
        let file_viewer = this.file_viewer_element();
        let directory_tree = this.directory_tree_element();
        file_viewer.innerHTML = `<header><div></div><div>Name</div><div>Size</div><div>Created</div><div>Last modified</div></header>`;
        directory_tree.innerHTML = "";
        for (let directory of response.body()) {
            if (directory.kind !== "directory")
                continue;
            let directory_tree_item = new DirectoryTreeItem(this, directory);
            let file_viewer_item = new FileViewerItem(this, directory);
            directory_tree.append(directory_tree_item.element);
            file_viewer.append(file_viewer_item.element);
        }
        for (let file of response.body()) {
            if (file.kind !== "file")
                continue;
            let file_viewer_item = new FileViewerItem(this, file);
            file_viewer.append(file_viewer_item.element);
        }
    }
    async __load_directory_tree() {
        let response = await HTTP.GET(`/v1/directory/${this.user()}/`)
            .Accept("application/json")
            .send();
        if (response.status() !== HTTPStatusCode.OK)
            throw new Error("Unable to retrieve directory.");
        let directory_tree = this.directory_tree_element();
        directory_tree.innerHTML = "";
        for (let directory of response.body()) {
            if (directory.kind !== "directory")
                continue;
            let directory_tree_item = new DirectoryTreeItem(this, directory);
            directory_tree.append(directory_tree_item.element);
        }
    }
    on_load() {
        let add_event_listener_to = (name, listener) => {
            this.root.querySelector(`button[name="${name}"]`)?.addEventListener("click", listener.bind(this));
        };
        add_event_listener_to("button_undo", this.on_undo);
        add_event_listener_to("button_redo", this.on_redo);
        add_event_listener_to("button_parent", this.on_parent);
        add_event_listener_to("button_breadcrumb_root", this.on_breadcrumb_root);
        add_event_listener_to("button_user_public", this.on_user_change);
        add_event_listener_to("button_user_self", this.on_user_change);
        add_event_listener_to("button_refresh", this.on_refresh);
        add_event_listener_to("button_new_folder", this.on_new_folder);
        add_event_listener_to("button_delete", this.on_delete);
        add_event_listener_to("button_select_all", this.on_select_all);
        add_event_listener_to("button_select_none", this.on_select_none);
        add_event_listener_to("button_select_inverse", this.on_select_inverse);
        for (let radio_view of this.root.querySelectorAll(`input[name="radio_view"]`)) {
            radio_view.addEventListener("click", this.on_view_change.bind(this));
        }
        this.file_viewer_element().parentElement?.addEventListener("click", this.on_file_viewer_click.bind(this));
        // @ts-ignore
        this.root.querySelector(`#splitFileExp`)?.addEventListener("drop", this.on_drop.bind(this));
        // @ts-ignore
        this.root.querySelector(`#splitFileExp`)?.addEventListener("dragover", this.on_dragover.bind(this));
        this.on_path_changed();
    }
    on_path_changed() {
        let user = this.path.current().user;
        let user_button = undefined;
        if (user === ".public") {
            user_button = this.root.querySelector(`button[name="button_user_public"]`);
            // @ts-ignore
        }
        else if (user === document['username']) {
            user_button = this.root.querySelector(`button[name="button_user_self"]`);
        }
        if (!user_button) {
            throw new Error("Current user not found in user list.");
        }
        for (let button of this.root.querySelectorAll(`button[name="button_breadcrumb_root"]`)) {
            if (button.innerHTML !== user_button.innerHTML) {
                this.__load_directory_tree()
                    .then(null);
            }
            button.innerHTML = user_button.innerHTML;
        }
        let undo_button = this.root.querySelector(`button[name="button_undo"]`);
        let redo_button = this.root.querySelector(`button[name="button_redo"]`);
        let parent_button = this.root.querySelector(`button[name="button_parent"]`);
        if (undo_button)
            undo_button.disabled = this.path.is_first();
        if (redo_button)
            redo_button.disabled = this.path.is_last();
        if (parent_button)
            parent_button.disabled = this.path.current().path === "/" || this.path.current().path === "";
        this.update_breadcrumb();
    }
    on_dragover(e) {
        if (e.dataTransfer)
            e.dataTransfer.dropEffect = "copy";
        e.preventDefault();
    }
    on_drop(e) {
        e.preventDefault();
        if (!e.dataTransfer)
            return;
        for (let file of e.dataTransfer.files) {
            this.upload_file(file)
                .then(console.log);
        }
    }
    on_refresh() {
        let tree = this.directory_tree();
        let touch = async (path, tree) => {
            let promise_list = [];
            for (let item in tree) {
                if (TreeNode.size(tree[item]) === 0)
                    continue;
                let element = this.directory_tree_by_path(`${path}/${item}`);
                let collapse_button = element
                    ?.previousElementSibling
                    ?.querySelector(":scope > i.bi-plus-square-dotted");
                if (!collapse_button)
                    continue;
                let resolver = (resolve) => {
                    let cb = collapse_button;
                    cb.addEventListener("content_load", async () => {
                        await touch(`${path}/${item}`, tree[item]);
                        resolve(null);
                    }, { once: true });
                    cb.click();
                };
                promise_list.push(new Promise(resolver));
            }
            await Promise.all(promise_list);
        };
        let loaded = async () => {
            await touch("", tree);
            await this.load_view()
                .then(this.on_path_changed.bind(this));
        };
        this.__load()
            .then(loaded);
    }
    on_new_folder() {
        let dialog_new_folder = document.querySelector(`#dialog_new_folder`);
        let button_create = dialog_new_folder.querySelector(`button[name="create"]`);
        let create_folder_event_handler = () => {
            let text_box = dialog_new_folder.querySelector(`input[name="folder_name"]`);
            let filename = text_box.value;
            this.create_folder(filename)
                .then(null);
        };
        dialog_new_folder.addEventListener("hidden.bs.modal", () => {
            setTimeout(() => {
                button_create.removeEventListener("click", create_folder_event_handler);
            }, 50);
        }, { once: true });
        button_create.addEventListener("click", create_folder_event_handler);
        let modal = new Modal(dialog_new_folder);
        modal.show();
    }
    on_delete() {
        let file_viewer = this.file_viewer_element();
        if (!file_viewer)
            return;
        let selected_items = file_viewer.querySelectorAll(`li.selected`);
        for (let item of selected_items) {
            let filename = item.querySelector("div.filename")?.textContent;
            if (!filename)
                continue;
            this.delete_file(filename)
                .then(null);
        }
    }
    on_selection_change() {
        let file_viewer = this.file_viewer_element();
        if (!file_viewer)
            return;
        let button_delete = this.root.querySelector(`button[name="button_delete"]`);
        if (!button_delete)
            return;
        button_delete.disabled = file_viewer.querySelector(`li.selected`) === null;
    }
    on_select_all() {
        let file_viewer = this.file_viewer_element();
        for (let file of file_viewer.children) {
            file.classList.add("selected");
        }
        this.on_selection_change();
    }
    on_select_inverse() {
        let file_viewer = this.file_viewer_element();
        for (let file of file_viewer.children) {
            file.classList.toggle("selected");
        }
        this.on_selection_change();
    }
    on_select_none() {
        let file_viewer = this.file_viewer_element();
        for (let file of file_viewer.children) {
            file.classList.remove("selected");
        }
        this.on_selection_change();
    }
    on_file_viewer_click(e) {
        if (!e.target)
            return;
        if (e.ctrlKey)
            return;
        let file_viewer = this.file_viewer_element();
        if (e.target !== file_viewer && e.target !== file_viewer.parentElement)
            return;
        for (let file of file_viewer.children) {
            file.classList.remove("selected");
        }
        this.on_selection_change();
    }
    on_view_change(e) {
        let sender = e.target;
        if (!sender)
            return;
        let file_viewer = this.file_viewer_element();
        file_viewer.classList.remove("large-icons", "medium-icons", "small-icons", "content", "list", "details");
        file_viewer.classList.add(sender.value);
    }
    on_undo() {
        this.path.go_back();
        this.load_view()
            .then(this.on_path_changed.bind(this));
    }
    on_redo() {
        this.path.go_forth();
        this.load_view()
            .then(this.on_path_changed.bind(this));
    }
    on_parent() {
        let path = this.path.current().path;
        path = path.split("/")
            .slice(0, -1)
            .join("/");
        this.navigate_to(new FullPath(this.path.current().user, path));
    }
    on_breadcrumb_root() {
        this.navigate_to(new FullPath(this.path.current().user, "/"));
    }
    on_user_change(e) {
        let sender = e.target;
        sender = sender?.closest("button");
        if (!sender)
            throw new Error("Button event has no sender.");
        let name = sender.getAttribute("name");
        let user = undefined;
        switch (name) {
            case "button_user_public":
                user = ".public";
                break;
            case "button_user_self":
                // @ts-ignore
                user = document['username'];
                break;
        }
        if (!user)
            throw new Error("Username not found.");
        if (user === this.path.current().user)
            return;
        this.navigate_to(new FullPath(user, "/"));
    }
}
function on_ready() {
    let file_explorer = new FileExplorer(".sf-component-file-explorer");
    document.addEventListener("keypress", (e) => {
        if (e.key === "Delete")
            file_explorer.on_delete();
    });
}
if (document.readyState === "complete") {
    on_ready();
}
else {
    document.onreadystatechange = on_ready;
}
//# sourceMappingURL=file_explorer.js.map