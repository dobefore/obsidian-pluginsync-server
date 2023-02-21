To begin with,copy code related to user authentication and user management and dependencies from anki-sync-server-rs .

construct a basic instance of actix-web server and implement the sync method host_key

In order to construct a actix-web server,I need to define a Server struct and a sync protocol
methods that will be implemented for the Server.

# constructure of the server 
