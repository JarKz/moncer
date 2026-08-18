# Project info

There stores the main idea of project and possible implementation of the application.

ToC:

  - [Architecture](#architecture)
  - [Loginless](#loginless)
  - [Database](#database)
    - [Categories](#categories)
    - [Transactions](#transactions)
    - [ORM](#orm)
  - [Server](#server)
  - [Clients](#clients)

## Architecture

The application intented to work as server and have changes only by a client.
The main reason of this choice is because I need to have centralized storage of money transactions.
Otherwise it'll be hard to track on different machines.

For instance, at one time I'll use my desktop machine to add a money transaction, then at another time
I'll use my laptop for the same task. Then I have two different histories of transactions and the merge
of them will be horrendous in almost all cases.

And the server will be simple at begin and can be extended with more features by new requirements.
The easy way to deal with requests is using RESTful pattern and it perfectly fits my task.

## Loginless

The application will have lack of authorization service because it intented to be bound to a single person.
If you want to have this, then use or implement an additional authorization service that will be like a wall
between client and server.

At most cases you don't need it if you use only machine to make transactions.
For dedicated server better to setup simple VPN and make the access by VPN IP address.
In this case unwanted users will have no access to application until you give them a way to connect to your VPN.

## Database

Instead of setuping the complex and powerful database like Cassandra, Postgres, MongoDB and so on,
the application will use very simple `SQLite`.

Since the application will be used in person, then it is enough to use very simple database and deal with him
in application level, if some features from other databases are missing in it.

### Categories

Each transaction will be divided into specific category and subcategory. It must help to understand where money
come from and to where they went. And it'll be just another table with an unique list of possible categories.

Each user can set their own categories and track in their way instead of providing a single way to define.
Not because a standartized way is bad, but other people's lives are different from mine and they might not
needed to have one category, and they want to have an additional one as well.

To avoid category explode with meaningless variants, they must be maintained carefully. Then need to force
users to make a think before doing a change.

Also custom categories may make hard to overview transactions within a particular period (like a month). So
it must not be a main feature, but the way that user can see the percentage of wasting money to unwanted category
or subcategory.

### Transactions

All transactions must be stored in database like a log information. In other words, each transaction will have
a datetime, money change, linked category and, possibly, a "reason".

With it a selection from table must be not hard because the `SQLite` database provide enough features to
correctly retrieve needed information.

About reason of transaction, there might be an unwanted transaction for an user and it's due to some good reason.
In this case an user can cut some "good" transaction, even they're unwanted though. Also it's good for them to
make a think, if there is a lot of unwanted waste of money but most of them have good reasons.

More importantly, need to research about correct way to store sensitive information like change. Because it
rarely is a round number, mostly float and using the `float`s (like `f32` or `f64`) will give an error. Instead
better to use some type that uses the integer and the decimal parts as both integers and operates with them together.

### ORM

In this project I'll use the Sea ORM to make a database, the migrations and actions with the database in
application. Currently I'm unsure is this final form of database because in future I may want to add something
helpful and the migrations are very important for this case.

The Sea ORM was chosen among others because I'm used to it. The complexity at start will be compensated by easy
maintenance in distance.

## Server

The application will be based on the `Actix` web framework because I'm used to it. It provides a lot of handful
ways to implement features, it's especially good for RESTful pattern.

Talking about endpoints, they must be very simple and do exactly one thing. Returned information may be in different
formats, and the standartized one is the JSON.

The main focus is minimalistic and low usage web service, because each appending or removing transaction happens
only sometimes.

And the server must provide a way to get compressed database for backups in case of server outage. I think there
are many helpful information that can be collected in several months or even in years and nobody wants to lose them.
And the restore from backup as well, but only in a manual way.

To make a backups without a thinking about it enough to setup a systemd service that runs by timers on client machine.
It's better if each machine have their own backups making a bulletproof way to avoid a data lose.

## Clients

There are many ways to implement clients. Even using a single `curl` utility is enough for all tasks, though. But it's
very inconvenient in distance, requiring to write a lot of information.

Instead of this, better to give a ready-to-use client for an user. At this moment it's enough to have a CLI utility,
because it fits all my requirements. But there might be plans to implement web-based client from a single HTML page.
