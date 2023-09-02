import { Collapse, Modal } from 'bootstrap';
import { HTTP } from '../../http/static/http.js';
import { Icon } from '../../http/static/themes';
/**
 * Kind of file descriptor.
 */
var DescriptorKind;
(function (DescriptorKind) {
    DescriptorKind["File"] = "file";
    DescriptorKind["Directory"] = "directory";
    DescriptorKind["None"] = "none";
})(DescriptorKind || (DescriptorKind = {}));
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
    if (waiter)
        waiter.remove();
}
/**
 * Sleeps for `timeout` milliseconds and then returns.
 *
 * @param timeout Number of milliseconds before returning.
 */
async function sleep(timeout) {
    return new Promise((resolve) => {
        setTimeout(resolve, timeout);
    });
}
/**
 * A `user/path` combination that has already been visited.
 *
 * It is used to store information for the "undo"/"redo" buttons in the file explorer.
 */
class PathHistoryItem {
    /**
     * User of the history item.
     */
    user;
    /**
     * Path of the history item.
     */
    path;
    constructor(user, path) {
        this.user = user;
        this.path = path;
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
     * Goes forth by one item, if possible, and returns the iten.
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
 * File explorer control.
 */
class FileExplorer {
    /**
     * HTML element showing the directory tree.
     */
    element_DirectoryTree;
    /**
     * HTML element showing the files in the current directory.
     */
    element_FileExplorer;
    /**
     * HTML element showing the current visited path.
     */
    element_Breadcrumbs;
    /**
     * History of visited paths for the "undo" and "redo" functionalities.
     */
    history;
    /**
     * Current path.
     */
    path;
    /**
     * Current user.
     */
    user;
    /**
     * List of descriptors of all the folders and files in the current location.
     */
    view;
    /**
     * Initializes the file explorer control.
     */
    constructor() {
        this.element_DirectoryTree = document.querySelector("#DirectoryTree");
        this.element_FileExplorer = document.querySelector("#FileViewer");
        this.element_Breadcrumbs = document.querySelector("#Breadcrumbs");
        this.user = document["username"];
        this.path = '/';
        this.history = new History(new PathHistoryItem(this.user, this.path));
        this.view = [];
        UIStartAwait();
        this.load_root()
            .then(this.on_load.bind(this));
    }
    /**
     * Creates a new HTMLElement representing an item in the directory tree.
     *
     * @param directory Descriptor of the directory.
     */
    create_directory_tree_item(directory) {
        let new_element_header_expand = document.createElement("i");
        let new_element_header_icon = document.createElement("img");
        let new_element_header_filename = document.createElement("span");
        let new_element_header = document.createElement("header");
        let new_element_body = document.createElement("ul");
        new_element_body.classList.add("tree-view", "collapse");
        if (directory.children > 0) {
            new_element_header_expand.classList.add("bi-plus-square-dotted");
            new_element_header_expand.addEventListener("click", this.on_directory_tree_item_collapse);
            for (let i = 0; i < directory.children; i++) {
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
        new_element_header_icon.src = "/static/ui/light/small/folder-vertical.png";
        new_element_header_icon.sizes = "(max-width: 16px) 16px, (max-width: 32px) 32px, 64px";
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
    /**
     * Creates a new HTMLElement representing an item in the file viewer.
     *
     * @param item Descriptor of the item.
     */
    create_file_viewer_item(item) {
        let new_element_filename = document.createElement("div");
        let new_element = document.createElement("li");
        if (item.kind === DescriptorKind.Directory) {
            let image_large = document.createElement("img");
            image_large.src = "/static/ui/light/large/folder.png";
            image_large.classList.add("icon-large");
            new_element.append(image_large);
            let image_medium = document.createElement("img");
            image_medium.src = "/static/ui/light/large/folder.png";
            image_medium.classList.add("icon-medium");
            new_element.append(image_medium);
            let image_small = document.createElement("img");
            image_small.src = "/static/ui/light/small/folder.png";
            image_small.classList.add("icon-small");
            new_element.append(image_small);
        }
        else {
            let extension = item.filename.split(".").slice(-1)[0];
            new Icon(extension)
                .apply_to(new_element);
        }
        new_element_filename.classList.add("filename");
        new_element_filename.innerText = item.filename;
        new_element.append(new_element_filename);
        new_element.addEventListener("dblclick", this.on_file_viewer_item_dblclick);
        return new_element;
    }
    /**
     * Navigates to a new directory, updating the HTML element.
     *
     * @param path Path of the directory.
     */
    navigate_to(path) {
        this.history.visit(new PathHistoryItem(this.user, path));
        let undo_button = document.querySelector("#FileExplorerButton_Undo");
        let redo_button = document.querySelector("#FileExplorerButton_Redo");
        let parent_button = document.querySelector("#FileExplorerButton_Parent");
        undo_button.disabled = false;
        redo_button.disabled = true;
        parent_button.disabled = path === "/" || path === "";
        this.path = path;
        this.load_file_viewer_view()
            .then(this.on_file_viewer_path_changed.bind(this));
    }
    /**
     * Returns the path of a directory tree item.
     *
     * @param element Element in the directory tree of which retrieve the path.
     */
    read_directory_tree_element_path(element) {
        let path = "";
        while (element) {
            while (element.tagName !== "LI" && element !== this.element_DirectoryTree) {
                element = element.parentElement;
                if (element === undefined)
                    break;
            }
            if (element === this.element_DirectoryTree)
                break;
            let element_filename = element.querySelector("header > span.filename");
            path = `/${element_filename.textContent}${path}`;
            element = element.parentElement;
        }
        return path;
    }
    /**
     * Finds the HTML element relative to the directory tree item of a given path.
     * Notice that this element is of type UL if the path is "/" and of type LI otherwise.
     *
     * @param path Path to look for in the directory tree.
     */
    seek_directory_tree_item(path) {
        let element = this.element_DirectoryTree;
        if (path === "/" || path === "")
            return element;
        if (path.startsWith("/"))
            path = path.slice(1);
        let exploded_path = path.split("/");
        let path_item = exploded_path.shift();
        while (path_item) {
            let next_element = null;
            for (let child of element.children) {
                let filename = child.querySelector("span.filename")
                    .textContent;
                if (filename !== path_item)
                    continue;
                path_item = exploded_path.shift();
                next_element = child.querySelector("ul.tree-view");
                if (path_item && !next_element)
                    return null;
                if (!path_item)
                    return child;
            }
            if (!next_element)
                return null;
            element = next_element;
        }
        return element;
    }
    /**
     * Changes the user relative to the file explorer.
     *
     * @param user Username of the new user.
     */
    set_user(user) {
        if (user === this.user)
            return;
        if (user === '.public') {
            document.querySelector("#FileExplorerButton_Root").innerHTML = document.querySelector("#FileExplorerButton_User_Public").innerHTML;
        }
        else {
            document.querySelector("#FileExplorerButton_Root").innerHTML = document.querySelector("#FileExplorerButton_User_Self").innerHTML;
        }
        this.user = user;
        this.load_root()
            .then(this.update_navigation_buttons.bind(this));
    }
    /**
     * Updates the navigation buttons depending on the state of the history and the current directory.
     */
    update_navigation_buttons() {
        let undo_button = document.querySelector("#FileExplorerButton_Undo");
        let redo_button = document.querySelector("#FileExplorerButton_Redo");
        let parent_button = document.querySelector("#FileExplorerButton_Parent");
        undo_button.disabled = this.history.is_first();
        redo_button.disabled = this.history.is_last();
        parent_button.disabled = this.path === "/" || this.path === "";
    }
    /**
     * Loads and shows the root directory for the current user.
     */
    async load_root() {
        let response = await HTTP.GET(`/v1/directory/${this.user}/`)
            .send();
        this.element_DirectoryTree.innerHTML = "";
        this.element_FileExplorer.innerHTML = "";
        for (let directory of response) {
            if (directory.kind !== "directory")
                continue;
            let new_directory_tree_element = this.create_directory_tree_item(directory);
            let new_file_viewer_element = this.create_file_viewer_item(directory);
            this.element_DirectoryTree.append(new_directory_tree_element);
            this.element_FileExplorer.append(new_file_viewer_element);
        }
        for (let file of response) {
            if (file.kind !== "file")
                continue;
            let new_file_viewer_element = this.create_file_viewer_item(file);
            this.element_FileExplorer.append(new_file_viewer_element);
        }
        this.view = response;
    }
    /**
     * Loads and shows all the child elements of a directory tree item.
     *
     * @param element HTML element of which load the child elements.
     */
    async load_directory_tree_children(element) {
        let path = this.read_directory_tree_element_path(element);
        let response = await HTTP.GET(`/v1/directory/${this.user}${path}`)
            .send();
        while (element.tagName !== "LI" && element !== this.element_DirectoryTree) {
            element = element.parentElement;
        }
        let element_body = element.querySelector("ul.tree-view");
        element_body.innerHTML = "";
        for (let directory of response) {
            if (directory.kind !== DescriptorKind.Directory)
                continue;
            let new_element = this.create_directory_tree_item(directory);
            element_body.append(new_element);
        }
        let element_collapse = element.querySelector("header > i");
        element_collapse.classList.remove("spinner-border", "spinner-border-sm");
        element_collapse.classList.add("bi-dash-square");
    }
    /**
     * Loads all the items in the current path and shows them in the file viewer.
     */
    async load_file_viewer_view() {
        let tree_view_path = this.seek_directory_tree_item(this.path);
        let response = await HTTP.GET(`/v1/directory/${this.user}${this.path}`)
            .send();
        this.element_FileExplorer.innerHTML = "";
        let element_DirectoryTreeItem = tree_view_path.querySelector("ul.tree-view");
        let element_CollapseButton = element_DirectoryTreeItem.previousElementSibling.querySelector("i");
        if (element_CollapseButton.classList.contains("bi-plus-square-dotted") && this.path !== "/" && this.path !== "") {
            element_DirectoryTreeItem.innerHTML = "";
            element_CollapseButton.classList.remove("bi-plus-square-dotted");
            element_CollapseButton.classList.add("bi-plus-square");
        }
        else {
            element_DirectoryTreeItem = null;
        }
        for (let file_item of response) {
            if (file_item.kind !== DescriptorKind.Directory)
                continue;
            let new_file_viewer_element = this.create_file_viewer_item(file_item);
            this.element_FileExplorer.append(new_file_viewer_element);
            if (element_DirectoryTreeItem) {
                let new_directory_tree_element = this.create_directory_tree_item(file_item);
                element_DirectoryTreeItem.append(new_directory_tree_element);
            }
        }
        for (let file_item of response) {
            if (file_item.kind !== DescriptorKind.File)
                continue;
            let new_element = this.create_file_viewer_item(file_item);
            this.element_FileExplorer.append(new_element);
        }
        this.view = response;
    }
    /**
     * Event triggered by clicking on an item in the breadcrumbs list.
     */
    on_breadcrumbs_item_click() {
        let sender = this;
        let file_explorer = document["file_explorer"];
        while (sender.tagName !== "LI" && sender !== file_explorer.element_Breadcrumbs) {
            sender = sender.parentElement;
        }
        let path = "";
        while (sender && sender.textContent !== "") {
            path = `/${sender.querySelector("a").textContent}${path}`;
            sender = sender.previousElementSibling;
        }
        file_explorer.navigate_to(path);
    }
    /**
     * Event triggered by clicking on the expand/collapse button in the directory tree viewer.
     */
    on_directory_tree_item_collapse() {
        let sender = this;
        let file_explorer = document["file_explorer"];
        if (sender.classList.contains("bi-dash-square")) {
            let element = sender.parentElement.nextElementSibling;
            new Collapse(element);
            sender.classList.remove("bi-dash-square");
            sender.classList.add("bi-plus-square");
            let adjacent_image = sender.nextElementSibling;
            adjacent_image.src = "/static/ui/light/small/folder-vertical.png";
        }
        else if (sender.classList.contains("bi-plus-square")) {
            let element = sender.parentElement.nextElementSibling;
            new Collapse(element);
            sender.classList.remove("bi-plus-square");
            sender.classList.add("bi-dash-square");
            let adjacent_image = sender.nextElementSibling;
            adjacent_image.src = "/static/ui/light/small/folder-vertical-open.png";
        }
        else if (sender.classList.contains("bi-plus-square-dotted")) {
            file_explorer.load_directory_tree_children(sender)
                .then();
            let element = sender.parentElement.nextElementSibling;
            new Collapse(element);
            sender.classList.remove("bi-plus-square-dotted");
            sender.classList.add("spinner-border", "spinner-border-sm");
            let adjacent_image = sender.nextElementSibling;
            adjacent_image.src = "/static/ui/light/small/folder-vertical-open.png";
        }
        return false;
    }
    /**
     * Event triggered by double-clicking an item in the directory tree viewer.
     */
    on_directory_tree_item_dblclick() {
        let sender = this;
        let file_explorer = document["file_explorer"];
        let path = file_explorer.read_directory_tree_element_path(sender);
        file_explorer.navigate_to(path);
    }
    /**
     * Event triggered by visiting another path.
     */
    on_file_viewer_path_changed() {
        let new_element = document.createElement("li");
        new_element.classList.add("breadcrumb-item");
        this.element_Breadcrumbs.innerHTML = "";
        this.element_Breadcrumbs.append(new_element);
        if (this.path == '/') {
            return;
        }
        let path = this.path.split("/");
        // Correction for first element being '/'
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
    /**
     * Event triggered by double-clicking an item in the file viewer.
     */
    on_file_viewer_item_dblclick() {
        let sender = this;
        let file_explorer = document["file_explorer"];
        while (sender.tagName !== "LI" && sender !== file_explorer.element_FileExplorer) {
            sender = sender.parentElement;
        }
        let filename = sender.querySelector("div.filename")
            .textContent;
        for (let file of file_explorer.view) {
            if (file.filename !== filename)
                continue;
            if (file.kind !== DescriptorKind.Directory) {
                // TODO: handle file recognition, etc.
                document["window_NotImplemented"].show();
                break;
            }
            let path = file_explorer.path;
            if (path === '/')
                path = '';
            file_explorer.navigate_to(`${path}/${filename}`);
            break;
        }
    }
    /**
     * Event triggered after loading the file explorer control.
     */
    on_load() {
        document.querySelector('#FileExplorerButton_Undo').addEventListener("click", this.on_undo_button_click);
        document.querySelector('#FileExplorerButton_Redo').addEventListener("click", this.on_redo_button_click);
        document.querySelector('#FileExplorerButton_Parent').addEventListener("click", this.on_parent_button_click);
        document.querySelector('#FileExplorerButton_Root').addEventListener("click", this.on_root_button_click);
        document.querySelector('#FileExplorerButton_User_Public').addEventListener("click", this.on_user_public_button_click);
        document.querySelector('#FileExplorerButton_User_Self').addEventListener("click", this.on_user_self_button_click);
        document.querySelector("#FileExplorer_Tabs_View_LargeIcons").addEventListener("click", this.on_view_radio_click);
        document.querySelector("#FileExplorer_Tabs_View_MediumIcons").addEventListener("click", this.on_view_radio_click);
        document.querySelector("#FileExplorer_Tabs_View_SmallIcons").addEventListener("click", this.on_view_radio_click);
        document.querySelector("#FileExplorer_Tabs_View_Content").addEventListener("click", this.on_view_radio_click);
        document.querySelector("#FileExplorer_Tabs_View_List").addEventListener("click", this.on_view_radio_click);
        document.querySelector("#FileExplorer_Tabs_View_Details").addEventListener("click", this.on_view_radio_click);
        document.querySelector("#FileExplorer_Tabs_Home_NewFolder").addEventListener("click", this.on_new_folder_button_click);
        document.querySelector("#FileExplorer_Tabs_Home_NewFolder").addEventListener("click", this.on_new_folder_button_click);
        document.querySelector("#Window_NewFolder_ButtonCreate").addEventListener("click", this.on_create_folder);
        this.element_FileExplorer.parentElement.addEventListener("drop", this.on_file_drop, false);
        this.element_FileExplorer.parentElement.addEventListener("dragover", this.on_file_drag_over, false);
        this.update_navigation_buttons();
        UIStopAwait();
    }
    /**
     * Event triggered by clicking on the "go to parent folder" button.
     */
    on_parent_button_click() {
        let file_explorer = document["file_explorer"];
        let exploded_path = file_explorer.path.split('/');
        exploded_path.pop();
        let path = exploded_path.join('/');
        file_explorer.navigate_to(path);
    }
    /**
     * Event triggered by clicking on the "go to root folder" button.
     */
    on_root_button_click() {
        let file_explorer = document["file_explorer"];
        file_explorer.navigate_to('/');
    }
    /**
     * Event triggered by clicking on the "public user" button.
     */
    on_user_public_button_click() {
        let file_explorer = document["file_explorer"];
        file_explorer.set_user('.public');
        file_explorer.navigate_to('/');
    }
    /**
     * Event triggered by clicking on the "current user" button.
     */
    on_user_self_button_click() {
        let file_explorer = document["file_explorer"];
        file_explorer.set_user(document["username"]);
        file_explorer.navigate_to('/');
    }
    /**
     * Event triggered by clicking on the "undo" button.
     */
    on_undo_button_click() {
        let file_explorer = document["file_explorer"];
        let item = file_explorer.history.go_back();
        file_explorer.set_user(item.user);
        file_explorer.path = item.path;
        file_explorer.update_navigation_buttons();
        file_explorer.load_file_viewer_view()
            .then(file_explorer.on_file_viewer_path_changed.bind(file_explorer));
    }
    /**
     * Event triggered by clicking on the "redo" button.
     */
    on_redo_button_click() {
        let file_explorer = document["file_explorer"];
        let item = file_explorer.history.go_forth();
        file_explorer.set_user(item.user);
        file_explorer.path = item.path;
        file_explorer.update_navigation_buttons();
        file_explorer.load_file_viewer_view()
            .then(file_explorer.on_file_viewer_path_changed.bind(file_explorer));
    }
    on_new_folder_button_click() {
        document["window_NewFolder"].show();
    }
    on_view_radio_click() {
        let file_explorer = document["file_explorer"];
        switch (this.id) {
            case "FileExplorer_Tabs_View_LargeIcons":
                file_explorer.element_FileExplorer.removeAttribute("class");
                file_explorer.element_FileExplorer.classList.add("file-view-icons-large");
                break;
            case "FileExplorer_Tabs_View_MediumIcons":
                file_explorer.element_FileExplorer.removeAttribute("class");
                file_explorer.element_FileExplorer.classList.add("file-view-icons-medium");
                break;
            case "FileExplorer_Tabs_View_SmallIcons":
                file_explorer.element_FileExplorer.removeAttribute("class");
                file_explorer.element_FileExplorer.classList.add("file-view-icons-small");
                break;
            case "FileExplorer_Tabs_View_Content":
                file_explorer.element_FileExplorer.removeAttribute("class");
                file_explorer.element_FileExplorer.classList.add("file-view-content");
                break;
            case "FileExplorer_Tabs_View_List":
                file_explorer.element_FileExplorer.removeAttribute("class");
                file_explorer.element_FileExplorer.classList.add("file-view-list");
                break;
            case "FileExplorer_Tabs_View_Details":
                file_explorer.element_FileExplorer.removeAttribute("class");
                file_explorer.element_FileExplorer.classList.add("file-view-details");
                break;
        }
    }
    on_create_folder() {
        let Window_NewFolder_Name = document.querySelector("#Window_NewFolder_Name");
        let file_explorer = document["file_explorer"];
        let filename = Window_NewFolder_Name.value;
        Window_NewFolder_Name.value = "";
        HTTP.POST(`/v1/directory/${file_explorer.user}${file_explorer.path}`)
            .body({ filename })
            .send()
            .then((data) => file_explorer.on_file_added.bind(file_explorer)(file_explorer.path, data));
    }
    on_file_added(path, data) {
        let directory_tree_parent = this.seek_directory_tree_item(path);
        let new_directory_tree_element = this.create_directory_tree_item(data);
        let new_file_viewer_element = this.create_file_viewer_item(data);
        this.element_FileExplorer.append(new_file_viewer_element);
        let previous_child = null;
        for (let child of directory_tree_parent.children) {
            if (child.querySelector("span.filename").textContent.toUpperCase() > data.filename.toUpperCase()) {
                previous_child = child.previousElementSibling;
                break;
            }
        }
        if (previous_child) {
            previous_child.after(new_directory_tree_element);
        }
        else {
            directory_tree_parent.append(new_directory_tree_element);
        }
        // TODO: resolve "cannot click" bug.
        // TODO: place sorted in file viewer.
    }
    on_file_drag_over(e) {
        e.dataTransfer.dropEffect = 'copy';
        e.preventDefault();
    }
    on_file_drop(e) {
        let file_explorer = document["file_explorer"];
        e.preventDefault();
        let new_item = document.createElement("li");
        let spinner_large = document.createElement("div");
        spinner_large.classList.add("spinner-border", "icon-large");
        spinner_large.style.width = "64px";
        spinner_large.style.height = "64px";
        spinner_large.style.margin = "auto";
        new_item.append(spinner_large);
        let spinner_medium = document.createElement("div");
        spinner_medium.classList.add("spinner-border", "icon-medium");
        spinner_medium.style.width = "32px";
        spinner_medium.style.height = "32px";
        spinner_medium.style.margin = "auto";
        new_item.append(spinner_medium);
        let spinner_small = document.createElement("div");
        spinner_small.classList.add("spinner-border", "icon-small");
        spinner_small.style.width = "16px";
        spinner_small.style.height = "16px";
        spinner_small.style.margin = "auto";
        new_item.append(spinner_small);
        let progress_container = document.createElement("div");
        progress_container.classList.add("progress");
        let progress_bar = document.createElement("div");
        progress_bar.classList.add("progress-bar", "progress-bar-striped", "progress-bar-animated");
        progress_bar.style.width = "50%";
        progress_bar.style.height = "1rem";
        progress_bar.setAttribute("name", "added_item");
        progress_container.append(progress_bar);
        new_item.append(progress_container);
        file_explorer.element_FileExplorer.append(new_item);
        let path = file_explorer.path;
        if (path === "/")
            path = "";
        for (let file of e.dataTransfer.files) {
            document["file_readers"][`${file_explorer.user}${path}/${file.name}`] = {
                file,
                reader: new FileReader()
            };
            upload_file(`${file_explorer.user}${path}`, file.name);
            console.log(file);
            console.log(`/v1/directory/${file_explorer.user}${path}/${file.name}`);
        }
    }
}
function upload_file(path, filename, slice_start = 0) {
    let slice_end = slice_start + 1048576 + 1;
    let blob = document["file_readers"][`${path}/${filename}`].file.slice(slice_start, slice_end);
    document["file_readers"][`${path}/${filename}`].reader.onloadend = async function (e) {
        if (e.target.readyState !== FileReader.DONE) {
            return;
        }
        let form_data = new FormData();
        form_data.append("filename", filename);
        form_data.append("content", blob);
        await HTTP.POST(`/v1/directory/${path}`)
            .content_type(false)
            .process_data(false)
            .body(form_data)
            .send();
        let percent_done = Math.floor((slice_end / document["file_readers"][`${path}/${filename}`].file.size) * 100);
        if (slice_end < document["file_readers"][`${path}/${filename}`].file.size) {
            console.log(`Progress: ${percent_done}%`);
            upload_file(path, filename, slice_end);
        }
        else {
            console.log("Upload completed!");
        }
    };
    document["file_readers"][`${path}/${filename}`].reader.readAsDataURL(blob);
}
JQ(() => {
    document["file_explorer"] = new FileExplorer();
    document["window_NotImplemented"] = new Modal("#Window_NotImplemented");
    document["window_NewFolder"] = new Modal("#Window_NewFolder");
    document["file_readers"] = [];
});
//# sourceMappingURL=directory.js.map