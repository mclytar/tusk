export class Result {
    ok;
    value;
    is_ok() {
        return this.ok;
    }
    is_err() {
        return !this.ok;
    }
    unwrap() {
        if (this.ok) {
            return this.value;
        }
        else {
            throw new Error("Tried to unwrap an error variant.");
        }
    }
    unwrap_err() {
        if (this.ok) {
            throw new Error("Tried to unwrap_err a non-error variant.");
        }
        else {
            return this.value;
        }
    }
}
function Ok(value) {
    let result = new Result();
    result.ok = true;
    result.value = value;
    return result;
}
function Err(err) {
    let result = new Result();
    result.ok = false;
    result.value = err;
    return result;
}
export var Method;
(function (Method) {
    Method["CONNECT"] = "CONNECT";
    Method["DELETE"] = "DELETE";
    Method["GET"] = "GET";
    Method["HEAD"] = "HEAD";
    Method["OPTIONS"] = "OPTIONS";
    Method["PATCH"] = "PATCH";
    Method["POST"] = "POST";
    Method["PUT"] = "PUT";
    Method["TRACE"] = "TRACE";
})(Method || (Method = {}));
export class HTTP2 {
    xhr;
    constructor(method, url) {
        this.xhr = new XMLHttpRequest();
        this.xhr.open(method, url);
    }
    CONNECT(url) {
        return new HTTP2(Method.CONNECT, url);
    }
    DELETE(url) {
        return new HTTP2(Method.DELETE, url);
    }
    GET(url) {
        return new HTTP2(Method.GET, url);
    }
    HEAD(url) {
        return new HTTP2(Method.HEAD, url);
    }
    OPTIONS(url) {
        return new HTTP2(Method.OPTIONS, url);
    }
    PATCH(url) {
        return new HTTP2(Method.PATCH, url);
    }
    POST(url) {
        return new HTTP2(Method.POST, url);
    }
    PUT(url) {
        return new HTTP2(Method.PUT, url);
    }
    TRACE(url) {
        return new HTTP2(Method.TRACE, url);
    }
    async send() {
        let response = new Promise((resolve) => {
            this.xhr.addEventListener("load", console.log);
        });
    }
}
document["HTTP"] = HTTP2;
//# sourceMappingURL=HTTP2.js.map