{OVERALL_GAME_HEADER}

<!-- 
--------
-- BGA framework: © Gregory Isabelli <gisabelli@boardgamearena.com> & Emmanuel Colin <ecolin@boardgamearena.com>
-- GuerreMagiciens implementation : © <Your name here> <Your email address here>
-- 
-- This code has been produced on the BGA studio platform for use on http://boardgamearena.com.
-- See http://en.boardgamearena.com/#!doc/Studio for more information.
-------

    guerremagiciens_guerremagiciens.tpl
    
    This is the HTML template of your game.
    
    Everything you are writing in this file will be displayed in the HTML page of your game user interface,
    in the "main game zone" of the screen.
    
    You can use in this template:
    _ variables, with the format {MY_VARIABLE_ELEMENT}.
    _ HTML block, with the BEGIN/END format
    
    See your "view" PHP file to check how to set variables and control blocks
    
    Please REMOVE this comment before publishing your game on BGA
-->


This is your game interface. You can edit this HTML in your ".tpl" file.


<script type="text/javascript">

// Javascript HTML templates

var jstpl_player_board = '\<div id="coins-frame">\
    <div id="coinsicon_p${id}" class="coin-icon"></div>\
    <div id="coinscount_p${id}" class="coin-text">${money}</div>\
</div>\
<div id="laboratories-frame">\
    <div id="laboratorycount_p${id}" class="laboratories-amount">${laboratories}</div>\
    <div id="laboratoryicon_p${id}" class="laboratory-icon"></div>\
</div>';

var jstpl_global_expense_input = '<div id="expense_token_t${id}" class="global-expense-coupon">${amount}</div>';

var jstpl_global_expense_sum = '<div id="expenses_total"><span class="">Total : </span><span id="global_expense_total" class="">0</span><div>';

var jstpl_produce_magical_items = '<div id="magical_item_t${id}" class="magical-item magical-item-${magical_item_name}">';

/*
// Example:
var jstpl_some_game_item='<div class="my_game_item" id="my_game_item_${id}"></div>';

*/

</script>  

<div id="board"></div>

<div id="right-area"></div>

{OVERALL_GAME_FOOTER}
