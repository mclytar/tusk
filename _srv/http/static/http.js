export var JQ = $;

export class HTTP {
    method;
    url;
    data = undefined;

    constructor(method, url) {
        this.method = method;
        this.url = url;
    }

    body(data) {
        this.data = data;
        return this;
    }

    async send() {
        let request = {};
        request.type = this.method;
        request.url = this.url;
        request.data = this.data;

        return JQ.ajax(request);
    }
}

HTTP.DELETE = function (url) {
    return new HTTP("DELETE", url);
}

HTTP.GET = function (url) {
    return new HTTP("GET", url);
}

HTTP.PATCH = function (url) {
    return new HTTP("PATCH", url);
}

HTTP.POST = function (url) {
    return new HTTP("POST", url);
}

HTTP.PUT = function (url) {
    return new HTTP("PUT", url);
}