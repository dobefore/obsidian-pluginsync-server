To begin with,copy code related to user authentication and user management and dependencies from anki-sync-server-rs .

construct a basic instance of actix-web server and implement the sync method host_key

In order to construct a actix-web server,I need to define a Server struct and a sync protocol
methods that will be implemented for the Server.

---
work on method hostkey
finish host_key.before doing it,first must create a server instance to hold the server's sync
state,Now the state just conations user info and its hostkey.

---
oh,need to write a handler to handle coming requests.
finish the basic handler,only able to handle host_key.

I build the server and let the client connect the server.connection is ok,but error appears when client parses the json the server sends.Oh I forgot to send hostkey response to client.
After fixing,hostkey is ok.
Now I try to put the plugin in phone,check whether it communicates with server normally in http mode.It seems working at least for host_key method and on my android 8.0

---
work on add a container SyncRequest for sync protocol methods

host key verification need being done in this method and beyond.In order to use header and nody of http request,must make a container to hold them all.Anki lib use SyncRequest,I adopt it.
This will change signatures of Syncprotocol methods.
Maybe my speed of key stroking is a bit slow,I have not finished revision.I will gave a break.
Continue to do reworking job.pack http header and body in a struct called SyncRequest.First,finish adjusting signatures of sync protocol methods.Oh signature adjust have already been done.

adjust code inside hosy_kry(),just use json method,other unaffected.Ok

---
finish mehod Server.with_authenticated_user()

Used to authenticate users in all methods but host_key.Check whther the key from the client has its place in a map of users from server database.By the way adding SyncRequest is for this.
finished. 

--- 
work on meta method.

labor with database to retrieve records.Create 2 tables ,meta including all sorts of file states such as ctime and content storing file contents.There is an index in meta that can retrieve content from content.
Create a struct for rust code to deserilize from db.store 
Snside the closure of with_authenticated_user lies code to handle meta request. 

How to determine a file exists on both sides?use loop to check.Use set methods,such as 
intersect and difference.

After a lot of efforts,finally made it.Now test it with client.First get 404,change Meta to meta in request http url. then get 500 I inspect the headers client sends.  "syncheadername": Value { inner: ["{\"k\":\"fc649a37298bbc83a00cf7cdefcd12f5c30f527ac68e59f038d297615ef3b4f6ca76cb3b63074805\"}"]

header name mismath.this time parse header json error oh the error is not here.

The problem lies in paring metaRequest.it is in client side.add metaRequest.
But another client receive a struct including undefined.field names must the same including cases of interfaces or structs.I am tired of this.

---
rework request parameters,such as f[],should add a wrapper for it to be the sane as that in server. make a change on method upload and download.
should create a test vault for testing use,in case my precious files are affected.
 
---
operation missing in meta.

When file delete,server mark them as delete in database.Ok

---
Work on upload
receieve files,store file meta and contents separately.Add path field to db meta.
Modify Upload request struct Pfile,add filestetes.

---
Work on download
retrieve file meta and content,send them to client.

---
adjust some request parameters in client method upload and download to be the same as server. yy

---
finish work adjusting meta code to allow for file stats and file events
but give error Json parsing error: missing field `action` at line 1 column 160.It is caused by out-dated client plugin code.

And some client-side error.
1. check folder exist before creating
2. check file exist before sppending file events to queue.

---
Uncaught (in promise) TypeError: Cannot read properties of undefined (reading 'mtime')
    at setFileInfo (file_event_type.ts:35:20)
    at async SyncPlugin.appendWatchEvent (main.ts:81:18)
    at async SyncPlugin.watchVaultCreate (main.ts:60:2)

This is caused when downloading files from server.In this method,client use create method,which will be caught as a create event,maybe no mtime.
So better to deal with this before append to queue.

---
some problem
files does not download to mobile.I assume that it is related to obsidian mobile verion,update to mobile 1.4.2. Even though installing latest version of obsidian,the behaviour is the same.

upload multiple copies of same file name

It's weird that sync does not work completely on android 1.4.0.It does not send http requests.

# Sync protocol methods
## meta
Tasks to do:
retrieve all file headers from server database .Compare them with ones from the client request.

If no files found on server,server mark client files Upload. 
If no files found on ckient aserver mark client files download.
??????????????????one file exists on client but not on server,mark it Upload?????????????????????
If no files on both sides.server sends empty vec.
if a file exists on both sides,mark it Modify  


mark some that does not exist on the server side as deleted.And send all sorts of file action
the client should take.
