export var JQ;
export type JQ = typeof JQ;

export enum Method {
    CONNECT = "CONNECT",
    DELETE = "DELETE",
    GET = "GET",
    HEAD = "HEAD",
    OPTIONS = "OPTIONS",
    PATCH = "PATCH",
    POST = "POST",
    PUT = "PUT",
    TRACE = "TRACE",
}

export class HTTP {
    method: Method;
    url: string;

    constructor(method: Method, url: string);

    public static DELETE(url: string): HTTP;
    public static GET(url: string): HTTP;
    public static PATCH(url: string): HTTP;
    public static POST(url: string): HTTP;
    public static PUT(url: string): HTTP;

    public body(data: any): HTTP;

    public send<T>() : Promise<T>;
}