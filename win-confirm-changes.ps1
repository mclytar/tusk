Remove-Item -Recurse .\_srv\*
Copy-Item -Path C:\srv\* -Destination .\_srv\ -Recurse
Remove-Item -Recurse .\_srv\.idea