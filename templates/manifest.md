# Pack Manifest

- Name: <%- name %>
- Date: <%- date %>
- Entries: <% wallpapers.len() %>
- Comments:
```
<%- comments %>
```

Title | Contributor | License
------|-------------|--------
<% for wallpaper in &wallpapers { %><%- wallpaper.title %> | <%- wallpaper.artist %> | <%- wallpaper.license %><% } %>
