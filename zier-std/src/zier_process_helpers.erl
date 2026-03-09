-module(zier_process_helpers).

-export([
    spawn/1,
    new_subject/0,
    named_subject/1,
    new_name/1,
    register/1,
    send/2,
    receive_timeout/2
]).

spawn(F) ->
    erlang:spawn(fun() -> F(unit) end).

new_subject() ->
    {subject, {subjectpayload, self(), make_ref()}}.

named_subject(Name) ->
    {namedsubject, Name}.

new_name(Name) when is_binary(Name) ->
    {name, Name, make_ref()}.

register({name, Name, Tag}) ->
    case global:register_name({zier_name, Name, Tag}, self()) of
        yes ->
            {ok, unit};
        no ->
            {error, <<"name already registered">>}
    end.

send({subject, {subjectpayload, Owner, Tag}}, Message) ->
    Owner ! {Tag, Message},
    Message;
send({namedsubject, {name, Name, Tag}}, Message) ->
    case global:whereis_name({zier_name, Name, Tag}) of
        undefined ->
            ok;
        Pid ->
            Pid ! {Tag, Message}
    end,
    Message.

receive_timeout({subject, {subjectpayload, _Owner, Tag}}, TimeoutMs) ->
    receive
        {Tag, Msg} -> {ok, Msg}
    after TimeoutMs ->
        {error, unit}
    end;
receive_timeout({namedsubject, {name, _Atom, Tag}}, TimeoutMs) ->
    receive_timeout({subject, {subjectpayload, self(), Tag}}, TimeoutMs).
